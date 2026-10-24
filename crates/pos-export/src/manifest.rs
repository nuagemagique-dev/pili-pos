//! Manifest of an export: what the files are and their SHA-256.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Value of [`Manifest::format`].
pub const FORMAT: &str = "pili-export/1";

/// One file listed in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    /// File name, without any directory.
    pub name: String,
    /// Size in bytes.
    pub bytes: i64,
    /// SHA-256, lowercase hex.
    pub sha256: String,
}

/// The `manifest-<id>.json` document (format `pili-export/1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// Always [`FORMAT`].
    pub format: String,
    pub export_id: String,
    /// RFC 3339 time of the export.
    pub created_at: String,
    pub pos_id: String,
    pub terminal_id: String,
    pub pos_sw_version: String,
    pub z_report_no: i64,
    /// `YYYY-MM-DD`.
    pub booking_date: String,
    pub stats_schema_version: i64,
    /// `age1...` recipients of the backup.
    pub recipients: Vec<String>,
    pub files: Vec<ManifestFile>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            format: FORMAT.to_string(),
            export_id: String::new(),
            created_at: String::new(),
            pos_id: String::new(),
            terminal_id: String::new(),
            pos_sw_version: String::new(),
            z_report_no: 0,
            booking_date: String::new(),
            stats_schema_version: 0,
            recipients: Vec::new(),
            files: Vec::new(),
        }
    }
}

/// SHA-256 of a file as lowercase hex, read as a stream.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    hash_and_size(path).map(|(hash, _)| hash)
}

fn hash_and_size(path: &Path) -> Result<(String, i64), String> {
    let mut file =
        File::open(path).map_err(|e| format!("sha256: cannot open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: i64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("sha256: cannot read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as i64;
    }
    Ok((hex::encode(hasher.finalize()), total))
}

impl Manifest {
    /// Write the manifest as pretty JSON, files sorted by name. Atomic: a
    /// temporary file in the same directory, synced, then renamed.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "manifest: format {:?} is not {FORMAT}",
                self.format
            ));
        }
        let mut sorted = self.clone();
        sorted.files.sort_by(|a, b| a.name.cmp(&b.name));
        let mut json =
            serde_json::to_string_pretty(&sorted).map_err(|e| format!("manifest: {e}"))?;
        json.push('\n');
        let mut tmp_name = path.as_os_str().to_os_string();
        tmp_name.push(".tmp");
        let tmp = PathBuf::from(tmp_name);
        let result = (|| {
            let mut f = File::create(&tmp)
                .map_err(|e| format!("manifest: cannot create {}: {e}", tmp.display()))?;
            f.write_all(json.as_bytes())
                .map_err(|e| format!("manifest: write: {e}"))?;
            f.sync_all().map_err(|e| format!("manifest: sync: {e}"))?;
            std::fs::rename(&tmp, path)
                .map_err(|e| format!("manifest: cannot rename {}: {e}", tmp.display()))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    /// Read a manifest; the format must be [`FORMAT`].
    pub fn read(path: &Path) -> Result<Manifest, String> {
        let raw = std::fs::read(path)
            .map_err(|e| format!("manifest: cannot read {}: {e}", path.display()))?;
        let m: Manifest = serde_json::from_slice(&raw)
            .map_err(|e| format!("manifest: invalid {}: {e}", path.display()))?;
        if m.format != FORMAT {
            return Err(format!("manifest: unknown format {:?}", m.format));
        }
        Ok(m)
    }

    /// Re-hash every listed file found in `dir`; the error names the first
    /// file that is missing, has another size or another SHA-256.
    pub fn verify(&self, dir: &Path) -> Result<(), String> {
        let mut files: Vec<&ManifestFile> = self.files.iter().collect();
        files.sort_by(|a, b| a.name.cmp(&b.name));
        for f in files {
            if f.name.is_empty() || f.name.contains(['/', '\\']) || f.name == "." || f.name == ".."
            {
                return Err(format!("file {:?}: not a plain file name", f.name));
            }
            let path = dir.join(&f.name);
            if !path.is_file() {
                return Err(format!("file {}: missing", f.name));
            }
            let (hash, bytes) = hash_and_size(&path)?;
            if bytes != f.bytes {
                return Err(format!(
                    "file {}: {bytes} bytes, {} expected",
                    f.name, f.bytes
                ));
            }
            if hash != f.sha256.to_ascii_lowercase() {
                return Err(format!("file {}: sha256 mismatch", f.name));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn sample(dir: &Path) -> Manifest {
        std::fs::write(dir.join("b.sqlite"), b"abc").unwrap();
        std::fs::write(dir.join("a.db.age"), b"hello").unwrap();
        Manifest {
            export_id: "20261008-z000042".into(),
            created_at: "2026-10-08T23:30:00+02:00".into(),
            pos_id: "CFOD0061234567".into(),
            terminal_id: "POS1".into(),
            pos_sw_version: "1.0.0".into(),
            z_report_no: 42,
            booking_date: "2026-10-08".into(),
            stats_schema_version: 1,
            recipients: vec!["age1x".into()],
            files: vec![
                ManifestFile {
                    name: "b.sqlite".into(),
                    bytes: 3,
                    sha256: sha256_file(&dir.join("b.sqlite")).unwrap(),
                },
                ManifestFile {
                    name: "a.db.age".into(),
                    bytes: 5,
                    sha256: sha256_file(&dir.join("a.db.age")).unwrap(),
                },
            ],
            ..Manifest::default()
        }
    }

    #[test]
    fn sha256_is_streamed_lowercase_hex() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_file(&p).unwrap(), ABC_SHA256);
        // Bigger than the read buffer.
        std::fs::write(&p, vec![7u8; 300_000]).unwrap();
        let expected = hex::encode(Sha256::digest(vec![7u8; 300_000]));
        assert_eq!(sha256_file(&p).unwrap(), expected);
        assert!(sha256_file(&dir.path().join("absent")).is_err());
    }

    #[test]
    fn write_then_read_round_trips_with_sorted_files() {
        let dir = tempfile::tempdir().unwrap();
        let m = sample(dir.path());
        let path = dir.path().join("manifest-20261008-z000042.json");
        m.write(&path).unwrap();
        assert!(
            !dir.path()
                .join("manifest-20261008-z000042.json.tmp")
                .exists()
        );
        let back = Manifest::read(&path).unwrap();
        assert_eq!(back.files[0].name, "a.db.age");
        assert_eq!(back.files[1].name, "b.sqlite");
        let mut sorted = m.clone();
        sorted.files.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(back, sorted);
    }

    #[test]
    fn the_json_is_camel_case_pretty_and_tagged_with_the_format() {
        let dir = tempfile::tempdir().unwrap();
        let m = sample(dir.path());
        let path = dir.path().join("m.json");
        m.write(&path).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"format\": \"pili-export/1\""));
        for key in [
            "exportId",
            "createdAt",
            "posId",
            "terminalId",
            "posSwVersion",
            "zReportNo",
            "bookingDate",
            "statsSchemaVersion",
            "recipients",
            "files",
            "sha256",
            "bytes",
        ] {
            assert!(raw.contains(&format!("\"{key}\"")), "{key} missing");
        }
        assert!(raw.contains('\n'));
    }

    #[test]
    fn a_wrong_format_is_refused_on_write_and_on_read() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = sample(dir.path());
        m.format = "pili-export/2".into();
        let path = dir.path().join("m.json");
        assert!(m.write(&path).is_err());
        assert!(!path.exists());
        let good = sample(dir.path());
        good.write(&path).unwrap();
        let raw = std::fs::read_to_string(&path)
            .unwrap()
            .replace("pili-export/1", "pili-export/9");
        std::fs::write(&path, raw).unwrap();
        assert!(
            Manifest::read(&path)
                .unwrap_err()
                .contains("unknown format")
        );
    }

    #[test]
    fn verify_accepts_intact_files() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path()).verify(dir.path()).unwrap();
    }

    #[test]
    fn verify_names_the_first_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let m = sample(dir.path());

        // Same size, other content: sha256 mismatch on the first name (a.db.age).
        std::fs::write(dir.path().join("a.db.age"), b"HELLO").unwrap();
        let err = m.verify(dir.path()).unwrap_err();
        assert!(err.contains("a.db.age") && err.contains("sha256"), "{err}");

        // Other size.
        std::fs::write(dir.path().join("a.db.age"), b"hello!").unwrap();
        let err = m.verify(dir.path()).unwrap_err();
        assert!(err.contains("a.db.age") && err.contains("bytes"), "{err}");

        // Restore the first, break the second: missing.
        std::fs::write(dir.path().join("a.db.age"), b"hello").unwrap();
        std::fs::remove_file(dir.path().join("b.sqlite")).unwrap();
        let err = m.verify(dir.path()).unwrap_err();
        assert!(err.contains("b.sqlite") && err.contains("missing"), "{err}");
    }

    #[test]
    fn verify_refuses_a_name_that_leaves_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = sample(dir.path());
        m.files[0].name = "../x".into();
        assert!(
            m.verify(dir.path())
                .unwrap_err()
                .contains("plain file name")
        );
    }
}
