//! Pseudonyms that replace personal data in the statistics file.
//!
//! The statistics file never holds an INSZ. A user is named there by
//! `user_ref`, a one-way pseudonym of the INSZ computed with a 32-byte key
//! that belongs to the installation. The key lives in its own file
//! (`pseudonym.key` in the exports directory), is created once, is never
//! exported and is not inside `pos.db`, so it is not in the encrypted backup
//! either. Whoever holds only the statistics file cannot go back to the INSZ
//! (the INSZ space is small, so without the key it could be enumerated), and
//! the same person keeps the same `user_ref` from one export to the next.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Name of the key file inside the exports directory.
pub const KEY_FILE: &str = "pseudonym.key";
/// Size of the key in bytes.
const KEY_LEN: usize = 32;
/// Domain tag of the hash: another use of the key can never collide.
const DOMAIN: &[u8] = b"pili-stats-user/1";
/// `user_ref` of the system identity (automatic Z, online orders).
pub const SYSTEM_REF: &str = "system";

/// Read `dir/pseudonym.key`, or create it with 32 bytes from the operating
/// system's randomness. The file is written beside its final name and linked
/// into place, so a reader never sees a partial key and a key that exists is
/// never replaced. On unix its mode is 0600. A file of another size is an
/// error and is left alone.
pub fn load_or_create_key(dir: &Path) -> Result<[u8; 32], String> {
    let path = dir.join(KEY_FILE);
    if let Some(key) = read_key(&path)? {
        return Ok(key);
    }
    fs::create_dir_all(dir).map_err(|e| format!("pseudonym key: cannot create directory: {e}"))?;
    let mut key = [0u8; KEY_LEN];
    getrandom::fill(&mut key).map_err(|e| format!("pseudonym key: no randomness: {e}"))?;
    let tmp = temp_path(dir);
    write_private(&tmp, &key)?;
    let linked = fs::hard_link(&tmp, &path);
    let _ = fs::remove_file(&tmp);
    match linked {
        Ok(()) => Ok(key),
        // Another thread or process created the key between our read and our link.
        Err(e) if e.kind() == ErrorKind::AlreadyExists => read_key(&path)?
            .ok_or_else(|| "pseudonym key: file vanished while being read".to_string()),
        // A file system without hard links: fall back to a rename, still atomic.
        Err(_) => {
            let tmp = temp_path(dir);
            write_private(&tmp, &key)?;
            if path.exists() {
                let _ = fs::remove_file(&tmp);
                return read_key(&path)?
                    .ok_or_else(|| "pseudonym key: file vanished while being read".to_string());
            }
            fs::rename(&tmp, &path).map_err(|e| {
                let _ = fs::remove_file(&tmp);
                format!("pseudonym key: cannot store the key: {e}")
            })?;
            Ok(key)
        }
    }
}

/// `Some(key)` for an existing well-formed file, `None` when there is no
/// file, an error for a file of another size.
fn read_key(path: &Path) -> Result<Option<[u8; 32]>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let key: [u8; KEY_LEN] = bytes.as_slice().try_into().map_err(|_| {
                format!(
                    "pseudonym key: {} holds {} bytes, {KEY_LEN} expected; refusing to replace it",
                    path.display(),
                    bytes.len()
                )
            })?;
            tighten(path);
            Ok(Some(key))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!(
            "pseudonym key: cannot read {}: {e}",
            path.display()
        )),
    }
}

fn temp_path(dir: &Path) -> PathBuf {
    let mut random = [0u8; 8];
    // The name only has to differ between two concurrent creators.
    let _ = getrandom::fill(&mut random);
    dir.join(format!(".{KEY_FILE}.{}.tmp", hex::encode(random)))
}

fn write_private(path: &Path, key: &[u8; KEY_LEN]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| format!("pseudonym key: cannot create {}: {e}", path.display()))?;
    let written = file.write_all(key).and_then(|()| file.sync_all());
    drop(file);
    written.map_err(|e| {
        let _ = fs::remove_file(path);
        format!("pseudonym key: cannot write {}: {e}", path.display())
    })
}

/// Make an existing key readable by its owner only (unix); best effort.
fn tighten(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// The pseudonym of the person with INSZ `insz`: `system` for
/// [`pos_core::user::SYSTEM_INSZ`], else `u-` and the first 16 lowercase hex
/// characters of SHA-256(`pili-stats-user/1` || key || INSZ).
pub fn user_ref(key: &[u8; 32], insz: &str) -> String {
    if insz == pos_core::user::SYSTEM_INSZ {
        return SYSTEM_REF.to_string();
    }
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(key);
    hasher.update(insz.as_bytes());
    let digest = hasher.finalize();
    format!("u-{}", &hex::encode(digest)[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_created_once_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create_key(dir.path()).unwrap();
        let again = load_or_create_key(dir.path()).unwrap();
        assert_eq!(first, again);
        assert_eq!(fs::read(dir.path().join(KEY_FILE)).unwrap(), first);
        assert_ne!(first, [0u8; 32], "the key comes from real randomness");
        // No temporary file is left behind.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(leftovers, vec![KEY_FILE.to_string()]);
    }

    #[test]
    fn two_installations_have_two_keys() {
        let a = load_or_create_key(tempfile::tempdir().unwrap().path()).unwrap();
        let b = load_or_create_key(tempfile::tempdir().unwrap().path()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_missing_directory_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("exports");
        load_or_create_key(&nested).unwrap();
        assert!(nested.join(KEY_FILE).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn the_key_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        load_or_create_key(dir.path()).unwrap();
        let mode = fs::metadata(dir.path().join(KEY_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn a_loose_key_file_is_tightened_not_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE);
        fs::write(&path, [7u8; 32]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(load_or_create_key(dir.path()).unwrap(), [7u8; 32]);
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_file_of_another_size_is_refused_and_kept() {
        for size in [0usize, 16, 31, 33, 64] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(KEY_FILE);
            fs::write(&path, vec![9u8; size]).unwrap();
            let err = load_or_create_key(dir.path()).unwrap_err();
            assert!(err.contains("refusing to replace"), "{size}: {err}");
            assert_eq!(fs::read(&path).unwrap(), vec![9u8; size], "{size}");
        }
    }

    #[test]
    fn the_pseudonym_is_stable_for_one_key_and_differs_for_another() {
        let key = [1u8; 32];
        let other = [2u8; 32];
        let a = user_ref(&key, "75061189702");
        assert_eq!(a, user_ref(&key, "75061189702"));
        assert_ne!(a, user_ref(&other, "75061189702"));
        assert_ne!(a, user_ref(&key, "84022899837"));
    }

    #[test]
    fn the_pseudonym_has_the_documented_shape_and_hides_the_insz() {
        let key = [3u8; 32];
        for insz in ["75061189702", "84022899837", "91051234567"] {
            let r = user_ref(&key, insz);
            assert_eq!(r.len(), 18, "{r}");
            let hex_part = r.strip_prefix("u-").expect("u- prefix");
            assert!(
                hex_part
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "{r}"
            );
            assert!(!r.contains(insz));
        }
    }

    #[test]
    fn the_system_identity_is_named_system() {
        assert_eq!(user_ref(&[4u8; 32], pos_core::user::SYSTEM_INSZ), "system");
    }

    #[test]
    fn the_pseudonym_follows_the_documented_formula() {
        let key = [5u8; 32];
        let mut input = b"pili-stats-user/1".to_vec();
        input.extend_from_slice(&key);
        input.extend_from_slice(b"75061189702");
        let expected = format!("u-{}", &hex::encode(Sha256::digest(&input))[..16]);
        assert_eq!(user_ref(&key, "75061189702"), expected);
    }
}
