//! Helpers the UI calls that need no engine: the ticket QR matrix, minisign verification and the
//! provisioning bundle check (PD61). Used by the provisioning flow (P4) and the updater (P6).
//!
//! Verification failures are values, never exceptions: [`MinisignCheck`] and [`BundleCheck`] carry
//! `valid` and a `problem` in the `code|k=v` form of the engine's error texts; the texts come with
//! the screens that show them. `PosError` is only for a call that could not run (a relative path,
//! a failed worker thread).
//!
//! Problem codes (PD61): `minisign_key_invalid`, `minisign_signature_invalid`,
//! `bundle_signature_invalid`, `bundle_manifest_invalid`, `bundle_path_invalid|path=`,
//! `bundle_file_missing|path=`, `bundle_size_mismatch|path=`, `bundle_hash_mismatch|path=`,
//! `bundle_unlisted_file|path=`. A bundle checked with an unusable public key reports
//! `minisign_key_invalid`; a missing `MANIFEST.json` is `bundle_manifest_invalid`, a missing or
//! unreadable `MANIFEST.json.minisig` is `bundle_signature_invalid`.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use minisign_verify::{PublicKey, Signature};
use sha2::{Digest, Sha256};

use crate::error::PosError;

/// File name of the signed manifest inside a bundle directory.
const MANIFEST: &str = "MANIFEST.json";
/// File name of the manifest signature inside a bundle directory.
const MANIFEST_SIG: &str = "MANIFEST.json.minisig";
/// The only manifest format this build understands.
const MANIFEST_FORMAT: i64 = 1;

/// Run `work` on a dedicated OS thread and await its result, so file work never runs on the
/// thread that polls the caller's future. A thread that cannot start or that panics is `Internal`.
pub(crate) async fn on_thread<T, F>(name: &str, work: F) -> Result<T, PosError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PosError> + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            // The receiver is gone when the caller was cancelled: nothing to report then.
            let _ = tx.send(work());
        })
        .map_err(|e| PosError::internal(format!("{name}: cannot start the worker thread: {e}")))?;
    rx.await
        .map_err(|_| PosError::internal(format!("{name}: the worker thread failed")))?
}

/// The ticket verification QR code as a square matrix of modules (`true` is dark).
#[uniffi::export]
pub async fn qr_matrix(text: String) -> Result<Vec<Vec<bool>>, PosError> {
    pos_fiscal::qr::generate_qr_matrix(&text)
        .map_err(|e| PosError::internal(format!("qr_matrix: {e}")))
}

/// The answer of [`verify_minisign`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct MinisignCheck {
    /// The signature is valid for the message under the public key.
    pub valid: bool,
    /// The signed trusted comment, when `valid`.
    pub trusted_comment: Option<String>,
    /// `minisign_key_invalid` or `minisign_signature_invalid`, when not `valid`.
    pub problem: Option<String>,
}

impl MinisignCheck {
    fn ok(trusted_comment: String) -> Self {
        Self {
            valid: true,
            trusted_comment: Some(trusted_comment),
            problem: None,
        }
    }

    fn failed(problem: &str) -> Self {
        Self {
            valid: false,
            trusted_comment: None,
            problem: Some(problem.to_string()),
        }
    }
}

/// The public key from its text: the base64 line alone, or the `minisign.pub` file with its
/// `untrusted comment:` line.
fn parse_public_key(text: &str) -> Option<PublicKey> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    match lines.as_slice() {
        [key] => PublicKey::from_base64(key).ok(),
        [comment, key] if comment.starts_with("untrusted comment:") => {
            PublicKey::from_base64(key).ok()
        }
        _ => None,
    }
}

/// Verify `signature` (the text of a `.minisig` file) over `message` with `public_key`. Returns
/// the signed trusted comment when valid. Only prehashed signatures (`minisign -S`, the default)
/// are accepted.
fn check_minisign(public_key: &str, message: &[u8], signature: &str) -> MinisignCheck {
    let Some(key) = parse_public_key(public_key) else {
        return MinisignCheck::failed("minisign_key_invalid");
    };
    let Ok(sig) = Signature::decode(signature) else {
        return MinisignCheck::failed("minisign_signature_invalid");
    };
    match key.verify(message, &sig, false) {
        Ok(()) => MinisignCheck::ok(sig.trusted_comment().to_string()),
        Err(_) => MinisignCheck::failed("minisign_signature_invalid"),
    }
}

/// Verify a minisign signature. `public_key` is the key text (the base64 line, the key file's
/// `untrusted comment:` line allowed), `signature` the text of the `.minisig` file. A bad key is
/// `minisign_key_invalid`; a bad, mismatching or foreign-key signature is
/// `minisign_signature_invalid`. Never an exception.
#[uniffi::export]
pub async fn verify_minisign(
    public_key: String,
    message: Vec<u8>,
    signature: String,
) -> MinisignCheck {
    check_minisign(&public_key, &message, &signature)
}

/// A file of a verified bundle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct BundleFile {
    /// Relative path, `/` separated.
    pub path: String,
    pub size: i64,
    /// Lowercase hexadecimal SHA-256.
    pub sha256: String,
}

/// The answer of [`verify_bundle`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct BundleCheck {
    /// The signature and every listed file match, and the directory holds nothing else.
    pub valid: bool,
    /// The verified files, when `valid`; empty otherwise.
    pub files: Vec<BundleFile>,
    /// The signed trusted comment of the manifest, when `valid`.
    pub trusted_comment: Option<String>,
    /// The first problem found (`code` or `code|path=<p>`), when not `valid`.
    pub problem: Option<String>,
}

impl BundleCheck {
    fn failed(problem: String) -> Self {
        Self {
            valid: false,
            files: Vec::new(),
            trusted_comment: None,
            problem: Some(problem),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: i64,
    files: Vec<ManifestFile>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    path: String,
    size: i64,
    sha256: String,
}

fn path_problem(path: &str) -> String {
    format!("bundle_path_invalid|path={path}")
}

/// Whether `path` is a plain relative path: at least one component, all of them names (no `..`,
/// no `.`, no root, no prefix), `/` separated, and not one of the two manifest files.
fn is_plain_relative(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') || path.contains('\0') {
        return false;
    }
    let p = Path::new(path);
    if p.is_absolute() {
        return false;
    }
    let mut count = 0usize;
    for c in p.components() {
        match c {
            Component::Normal(_) => count += 1,
            _ => return false,
        }
    }
    // `a//b` and a trailing `/` collapse in `components`: refuse any other spelling.
    if count == 0 || path.split('/').any(str::is_empty) {
        return false;
    }
    path != MANIFEST && path != MANIFEST_SIG
}

/// SHA-256 and size of the regular file at `path`, read in chunks.
fn hash_file(path: &Path) -> std::io::Result<(u64, String)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((total, hex::encode(hasher.finalize())))
}

/// Every non-directory entry under `root` as a `/` separated relative path, symlinks and other
/// special files included (they are never followed).
fn walk(root: &Path, rel: &mut Vec<String>, out: &mut Vec<String>) -> std::io::Result<()> {
    let dir: PathBuf = rel.iter().fold(root.to_path_buf(), |p, c| p.join(c));
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;
        rel.push(name);
        if kind.is_dir() {
            walk(root, rel, out)?;
        } else {
            out.push(rel.join("/"));
        }
        rel.pop();
    }
    Ok(())
}

/// The first problem of the bundle in `dir`, or the verified result. I/O failures are `Err`.
fn check_bundle(dir: &Path, public_key: &str) -> Result<BundleCheck, PosError> {
    let io =
        |what: &str, e: std::io::Error| PosError::internal(format!("verify_bundle: {what}: {e}"));

    let manifest = match std::fs::read(dir.join(MANIFEST)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BundleCheck::failed("bundle_manifest_invalid".into()));
        }
        Err(e) => return Err(io(MANIFEST, e)),
    };
    let signature = match std::fs::read_to_string(dir.join(MANIFEST_SIG)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BundleCheck::failed("bundle_signature_invalid".into()));
        }
        // Not UTF-8: not a minisign signature.
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(BundleCheck::failed("bundle_signature_invalid".into()));
        }
        Err(e) => return Err(io(MANIFEST_SIG, e)),
    };

    let trusted_comment = match check_minisign(public_key, &manifest, &signature) {
        MinisignCheck {
            valid: true,
            trusted_comment,
            ..
        } => trusted_comment,
        MinisignCheck {
            problem: Some(p), ..
        } if p == "minisign_key_invalid" => {
            return Ok(BundleCheck::failed(p));
        }
        _ => return Ok(BundleCheck::failed("bundle_signature_invalid".into())),
    };

    let parsed: Manifest = match serde_json::from_slice::<Manifest>(&manifest) {
        Ok(m) if m.format == MANIFEST_FORMAT => m,
        _ => return Ok(BundleCheck::failed("bundle_manifest_invalid".into())),
    };

    let mut listed: HashSet<&str> = HashSet::new();
    let mut files = Vec::with_capacity(parsed.files.len());
    for entry in &parsed.files {
        let rel = entry.path.as_str();
        if !is_plain_relative(rel) {
            return Ok(BundleCheck::failed(path_problem(rel)));
        }
        if !listed.insert(rel) {
            // The same path twice: the manifest is ambiguous.
            return Ok(BundleCheck::failed("bundle_manifest_invalid".into()));
        }
        // No symlink anywhere on the path (the bundle directory itself is the caller's).
        let mut current = dir.to_path_buf();
        let parts: Vec<&str> = rel.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            current.push(part);
            let meta = match std::fs::symlink_metadata(&current) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(BundleCheck::failed(format!(
                        "bundle_file_missing|path={rel}"
                    )));
                }
                Err(e) => return Err(io(rel, e)),
            };
            let last = i + 1 == parts.len();
            let kind = meta.file_type();
            if kind.is_symlink() || (last && !kind.is_file()) || (!last && !kind.is_dir()) {
                return Ok(BundleCheck::failed(path_problem(rel)));
            }
        }
        if u64::try_from(entry.size).ok() != std::fs::metadata(&current).ok().map(|m| m.len()) {
            return Ok(BundleCheck::failed(format!(
                "bundle_size_mismatch|path={rel}"
            )));
        }
        let (size, digest) = hash_file(&current).map_err(|e| io(rel, e))?;
        if i64::try_from(size).ok() != Some(entry.size) {
            // The file changed between the two reads.
            return Ok(BundleCheck::failed(format!(
                "bundle_size_mismatch|path={rel}"
            )));
        }
        if digest != entry.sha256 {
            return Ok(BundleCheck::failed(format!(
                "bundle_hash_mismatch|path={rel}"
            )));
        }
        files.push(BundleFile {
            path: rel.to_string(),
            size: entry.size,
            sha256: digest,
        });
    }

    let mut present = Vec::new();
    walk(dir, &mut Vec::new(), &mut present).map_err(|e| io("listing the bundle", e))?;
    present.sort();
    for rel in present {
        if rel == MANIFEST || rel == MANIFEST_SIG || listed.contains(rel.as_str()) {
            continue;
        }
        return Ok(BundleCheck::failed(format!(
            "bundle_unlisted_file|path={rel}"
        )));
    }

    Ok(BundleCheck {
        valid: true,
        files,
        trusted_comment,
        problem: None,
    })
}

/// Verify the provisioning bundle in the absolute directory `dir` against `public_key` (PD61):
/// the signature over the exact bytes of `MANIFEST.json`, then every listed file (plain relative
/// path, no symlink, size and SHA-256 equal), then no unlisted file. Stops at the first problem.
/// The file work runs on a dedicated thread.
#[uniffi::export]
pub async fn verify_bundle(dir: String, public_key: String) -> Result<BundleCheck, PosError> {
    let path = PathBuf::from(&dir);
    if !path.is_absolute() {
        return Err(PosError::internal(format!(
            "verify_bundle: dir must be an absolute path: {dir}"
        )));
    }
    on_thread("pos-ffi-bundle", move || check_bundle(&path, &public_key)).await
}
