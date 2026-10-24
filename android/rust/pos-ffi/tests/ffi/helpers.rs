//! Tests of the pure helpers: the QR matrix, minisign verification and the bundle check (PD61).
//! Every key and signature is generated here; nothing signed is committed.

use std::io::Cursor;
use std::path::Path;

use minisign::KeyPair;
use pos_ffi::helpers::{
    BundleCheck, BundleFile, MinisignCheck, qr_matrix, verify_bundle, verify_minisign,
};
use sha2::{Digest, Sha256};

const TRUSTED: &str = "timestamp:1790000000\tfile:bundle";

/// A throwaway signing key, never written to disk.
struct Signer {
    pair: KeyPair,
}

impl Signer {
    fn new() -> Self {
        Self {
            pair: KeyPair::generate_unencrypted_keypair().expect("generate a key pair"),
        }
    }

    /// The key as the `minisign.pub` file text (with its `untrusted comment:` line).
    fn public_text(&self) -> String {
        self.pair.pk.to_box().expect("public key box").into_string()
    }

    /// The key as the bare base64 line.
    fn public_base64(&self) -> String {
        self.pair.pk.to_base64()
    }

    fn sign(&self, message: &[u8]) -> String {
        minisign::sign(
            Some(&self.pair.pk),
            &self.pair.sk,
            Cursor::new(message),
            Some(TRUSTED),
            None,
        )
        .expect("sign")
        .into_string()
    }
}

fn problem(check: &MinisignCheck) -> &str {
    assert!(!check.valid, "{check:?}");
    assert_eq!(check.trusted_comment, None);
    check.problem.as_deref().expect("a problem")
}

// ---- qr_matrix

#[tokio::test]
async fn qr_matrix_is_a_square_deterministic_matrix_equal_to_the_ticket_encoder() {
    let url = "https://fdm.example.test/v/ABCDEFGH1234567890abcdefgh12345678";
    let matrix = qr_matrix(url.to_string()).await.expect("qr");
    assert!(!matrix.is_empty());
    assert!(matrix.iter().all(|row| row.len() == matrix.len()), "square");
    assert!(matrix.iter().flatten().any(|m| *m), "has dark modules");
    assert!(matrix.iter().flatten().any(|m| !*m), "has light modules");
    assert_eq!(matrix, qr_matrix(url.to_string()).await.unwrap());
    assert_eq!(matrix, pos_fiscal::qr::generate_qr_matrix(url).unwrap());
}

#[tokio::test]
async fn qr_matrix_of_the_empty_text_still_answers() {
    // Whatever the encoder does with it, the facade agrees with it (an error is Internal).
    match (
        qr_matrix(String::new()).await,
        pos_fiscal::qr::generate_qr_matrix(""),
    ) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(pos_ffi::PosError::Internal { .. }), Err(_)) => {}
        (a, b) => panic!("the facade and the encoder disagree: {a:?} / {b:?}"),
    }
}

// ---- verify_minisign

#[tokio::test]
async fn minisign_accepts_a_good_signature_with_either_key_format() {
    let signer = Signer::new();
    let message = b"the artefact bytes".to_vec();
    let signature = signer.sign(&message);
    for key in [signer.public_text(), signer.public_base64()] {
        let check = verify_minisign(key, message.clone(), signature.clone()).await;
        assert_eq!(
            check,
            MinisignCheck {
                valid: true,
                trusted_comment: Some(TRUSTED.to_string()),
                problem: None,
            }
        );
    }
}

#[tokio::test]
async fn minisign_reports_a_tampered_message() {
    let signer = Signer::new();
    let signature = signer.sign(b"original");
    let check = verify_minisign(signer.public_text(), b"originaL".to_vec(), signature).await;
    assert_eq!(problem(&check), "minisign_signature_invalid");
}

#[tokio::test]
async fn minisign_reports_a_signature_of_another_key() {
    let signer = Signer::new();
    let other = Signer::new();
    let signature = signer.sign(b"data");
    let check = verify_minisign(other.public_text(), b"data".to_vec(), signature).await;
    assert_eq!(problem(&check), "minisign_signature_invalid");
}

#[tokio::test]
async fn minisign_reports_a_garbage_key() {
    let signer = Signer::new();
    let signature = signer.sign(b"data");
    for key in ["", "not a key", "untrusted comment: x\n!!!!", "AAAA"] {
        let check = verify_minisign(key.to_string(), b"data".to_vec(), signature.clone()).await;
        assert_eq!(problem(&check), "minisign_key_invalid", "key {key:?}");
    }
}

#[tokio::test]
async fn minisign_reports_a_garbage_signature() {
    let signer = Signer::new();
    for sig in [
        "",
        "garbage",
        "untrusted comment: x\nAAAA\ntrusted comment: y\nAAAA",
    ] {
        let check = verify_minisign(signer.public_text(), b"data".to_vec(), sig.to_string()).await;
        assert_eq!(
            problem(&check),
            "minisign_signature_invalid",
            "signature {sig:?}"
        );
    }
}

#[tokio::test]
async fn minisign_reports_a_tampered_trusted_comment() {
    let signer = Signer::new();
    let signature = signer
        .sign(b"data")
        .replace(TRUSTED, "timestamp:1\tfile:other");
    let check = verify_minisign(signer.public_text(), b"data".to_vec(), signature).await;
    assert_eq!(problem(&check), "minisign_signature_invalid");
}

// ---- verify_bundle

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

const BUNDLE_FILES: [(&str, &str); 3] = [
    ("pos.json", "{\"dummy\":true}"),
    ("fdm.token", "token-for-tests"),
    (
        "tls/ca.pem",
        "-----BEGIN CERTIFICATE-----\ndummy\n-----END CERTIFICATE-----\n",
    ),
];

fn manifest_for(files: &[(&str, &str)]) -> String {
    let entries: Vec<String> = files
        .iter()
        .map(|(path, body)| {
            format!(
                r#"{{"path":{path:?},"size":{},"sha256":"{}"}}"#,
                body.len(),
                sha256_hex(body.as_bytes())
            )
        })
        .collect();
    format!(r#"{{"format":1,"files":[{}]}}"#, entries.join(","))
}

fn write_file(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// Write the files of `BUNDLE_FILES`, then sign `manifest` and write both manifest files.
fn write_bundle(dir: &Path, signer: &Signer, manifest: &str) {
    for (rel, body) in BUNDLE_FILES {
        write_file(dir, rel, body);
    }
    write_manifest(dir, signer, manifest);
}

fn write_manifest(dir: &Path, signer: &Signer, manifest: &str) {
    std::fs::write(dir.join("MANIFEST.json"), manifest).unwrap();
    std::fs::write(
        dir.join("MANIFEST.json.minisig"),
        signer.sign(manifest.as_bytes()),
    )
    .unwrap();
}

fn good_bundle() -> (tempfile::TempDir, Signer) {
    let dir = tempfile::tempdir().unwrap();
    let signer = Signer::new();
    write_bundle(dir.path(), &signer, &manifest_for(&BUNDLE_FILES));
    (dir, signer)
}

async fn check(dir: &Path, signer: &Signer) -> BundleCheck {
    verify_bundle(dir.to_string_lossy().into_owned(), signer.public_text())
        .await
        .expect("the check ran")
}

fn bundle_problem(check: &BundleCheck) -> &str {
    assert!(!check.valid, "{check:?}");
    assert!(check.files.is_empty());
    assert_eq!(check.trusted_comment, None);
    check.problem.as_deref().expect("a problem")
}

/// Sign `manifest` (replacing the good one) and check the bundle.
async fn check_with_manifest(dir: &Path, signer: &Signer, manifest: &str) -> BundleCheck {
    write_manifest(dir, signer, manifest);
    check(dir, signer).await
}

#[tokio::test]
async fn a_good_bundle_is_valid_with_its_three_files() {
    let (dir, signer) = good_bundle();
    let result = check(dir.path(), &signer).await;
    assert!(result.valid, "{result:?}");
    assert_eq!(result.problem, None);
    assert_eq!(result.trusted_comment.as_deref(), Some(TRUSTED));
    let expected: Vec<BundleFile> = BUNDLE_FILES
        .iter()
        .map(|(path, body)| BundleFile {
            path: path.to_string(),
            size: body.len() as i64,
            sha256: sha256_hex(body.as_bytes()),
        })
        .collect();
    assert_eq!(result.files, expected);
    // The bare base64 key works too.
    let again = verify_bundle(
        dir.path().to_string_lossy().into_owned(),
        signer.public_base64(),
    )
    .await
    .unwrap();
    assert!(again.valid);
}

#[tokio::test]
async fn a_relative_directory_is_refused() {
    let signer = Signer::new();
    let err = verify_bundle("relative/dir".into(), signer.public_text())
        .await
        .unwrap_err();
    assert!(matches!(err, pos_ffi::PosError::Internal { .. }), "{err:?}");
}

#[tokio::test]
async fn a_tampered_manifest_is_a_signature_problem() {
    let (dir, signer) = good_bundle();
    let tampered = manifest_for(&BUNDLE_FILES).replace("\"size\":14", "\"size\":15");
    assert_ne!(tampered, manifest_for(&BUNDLE_FILES));
    std::fs::write(dir.path().join("MANIFEST.json"), tampered).unwrap();
    let result = check(dir.path(), &signer).await;
    assert_eq!(bundle_problem(&result), "bundle_signature_invalid");
}

#[tokio::test]
async fn a_missing_signature_file_is_a_signature_problem() {
    let (dir, signer) = good_bundle();
    std::fs::remove_file(dir.path().join("MANIFEST.json.minisig")).unwrap();
    let result = check(dir.path(), &signer).await;
    assert_eq!(bundle_problem(&result), "bundle_signature_invalid");
}

#[tokio::test]
async fn a_missing_manifest_is_a_manifest_problem() {
    let (dir, signer) = good_bundle();
    std::fs::remove_file(dir.path().join("MANIFEST.json")).unwrap();
    let result = check(dir.path(), &signer).await;
    assert_eq!(bundle_problem(&result), "bundle_manifest_invalid");
}

#[tokio::test]
async fn the_wrong_key_is_a_signature_problem_and_a_garbage_key_a_key_problem() {
    let (dir, _signer) = good_bundle();
    let other = Signer::new();
    let result = check(dir.path(), &other).await;
    assert_eq!(bundle_problem(&result), "bundle_signature_invalid");
    let result = verify_bundle(dir.path().to_string_lossy().into_owned(), "garbage".into())
        .await
        .unwrap();
    assert_eq!(bundle_problem(&result), "minisign_key_invalid");
}

#[tokio::test]
async fn another_format_or_bad_json_is_a_manifest_problem() {
    let (dir, signer) = good_bundle();
    let good = manifest_for(&BUNDLE_FILES);
    for manifest in [
        good.replace("\"format\":1", "\"format\":2"),
        "not json".to_string(),
        r#"{"format":1}"#.to_string(),
        good.replace("\"files\"", "\"extra\":1,\"files\""),
        // The same path twice is ambiguous.
        manifest_for(&[
            BUNDLE_FILES[0],
            BUNDLE_FILES[0],
            BUNDLE_FILES[1],
            BUNDLE_FILES[2],
        ]),
    ] {
        let result = check_with_manifest(dir.path(), &signer, &manifest).await;
        assert_eq!(
            bundle_problem(&result),
            "bundle_manifest_invalid",
            "{manifest}"
        );
    }
}

#[tokio::test]
async fn paths_that_escape_or_are_not_plain_are_refused() {
    let (dir, signer) = good_bundle();
    // A real file next to the bundle: `..` must be refused even when the target exists.
    let outer = tempfile::tempdir().unwrap();
    let inner = outer.path().join("bundle");
    std::fs::create_dir_all(&inner).unwrap();
    write_bundle(&inner, &signer, &manifest_for(&BUNDLE_FILES));
    std::fs::write(outer.path().join("x"), "outside").unwrap();

    for (base, path) in [
        (dir.path(), "../x"),
        (inner.as_path(), "../x"),
        (dir.path(), "tls/../pos.json"),
        (dir.path(), "/etc/passwd"),
        (dir.path(), "./pos.json"),
        (dir.path(), "tls//ca.pem"),
        (dir.path(), "tls/"),
        (dir.path(), "tls\\ca.pem"),
        (dir.path(), ""),
        (dir.path(), "MANIFEST.json"),
    ] {
        let manifest = manifest_for(&[(path, "x")]);
        let result = check_with_manifest(base, &signer, &manifest).await;
        assert_eq!(
            bundle_problem(&result),
            format!("bundle_path_invalid|path={path}"),
            "path {path:?}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_symlinked_file_or_directory_is_refused() {
    let (dir, signer) = good_bundle();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), "x").unwrap();

    // A symlink as the file itself.
    std::os::unix::fs::symlink(outside.path().join("secret"), dir.path().join("link")).unwrap();
    let result = check_with_manifest(dir.path(), &signer, &manifest_for(&[("link", "x")])).await;
    assert_eq!(bundle_problem(&result), "bundle_path_invalid|path=link");

    // A symlink as a directory on the path.
    std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).unwrap();
    let result =
        check_with_manifest(dir.path(), &signer, &manifest_for(&[("sub/secret", "x")])).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_path_invalid|path=sub/secret"
    );
}

#[tokio::test]
async fn a_missing_file_is_reported_with_its_path() {
    let (dir, signer) = good_bundle();
    std::fs::remove_file(dir.path().join("tls/ca.pem")).unwrap();
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_file_missing|path=tls/ca.pem"
    );
}

#[tokio::test]
async fn a_different_size_is_reported_with_its_path() {
    let (dir, signer) = good_bundle();
    write_file(dir.path(), "fdm.token", "token-for-tests-and-more");
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_size_mismatch|path=fdm.token"
    );
}

#[tokio::test]
async fn a_different_content_of_the_same_size_is_a_hash_problem() {
    let (dir, signer) = good_bundle();
    // Same size as the manifest says (15 bytes), other bytes.
    write_file(dir.path(), "fdm.token", "token-for-tezts");
    assert_eq!("token-for-tests".len(), "token-for-tezts".len());
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_hash_mismatch|path=fdm.token"
    );
}

#[tokio::test]
async fn an_uppercase_digest_in_the_manifest_is_a_hash_problem() {
    let (dir, signer) = good_bundle();
    let good = manifest_for(&BUNDLE_FILES);
    let digest = sha256_hex(BUNDLE_FILES[0].1.as_bytes());
    let manifest = good.replace(&digest, &digest.to_uppercase());
    let result = check_with_manifest(dir.path(), &signer, &manifest).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_hash_mismatch|path=pos.json"
    );
}

#[tokio::test]
async fn a_file_the_manifest_does_not_list_is_refused() {
    let (dir, signer) = good_bundle();
    write_file(dir.path(), "tls/extra.pem", "surprise");
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_unlisted_file|path=tls/extra.pem"
    );

    // An unlisted file at the top level.
    std::fs::remove_file(dir.path().join("tls/extra.pem")).unwrap();
    write_file(dir.path(), "notes.txt", "surprise");
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_unlisted_file|path=notes.txt"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn an_unlisted_symlink_is_refused() {
    let (dir, signer) = good_bundle();
    std::os::unix::fs::symlink("pos.json", dir.path().join("alias")).unwrap();
    let result = check(dir.path(), &signer).await;
    assert_eq!(bundle_problem(&result), "bundle_unlisted_file|path=alias");
}

#[tokio::test]
async fn the_first_problem_wins() {
    let (dir, signer) = good_bundle();
    write_file(dir.path(), "extra", "x");
    std::fs::remove_file(dir.path().join("fdm.token")).unwrap();
    // The missing listed file comes before the unlisted file.
    let result = check(dir.path(), &signer).await;
    assert_eq!(
        bundle_problem(&result),
        "bundle_file_missing|path=fdm.token"
    );
}
