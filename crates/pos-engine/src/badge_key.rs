//! The installation's badge key and the keyed hash of a badge number.
//!
//! A badge number (the UID of a card or key fob, 4 to 7 bytes) is a
//! small space: stored as a plain hash it could be enumerated from a
//! stolen database. The engine stores only
//! `HMAC-SHA256(key, "pili-badge/1\n" || token)`, with a 32-byte key
//! that belongs to the installation. The key lives in `badge.key` in the
//! data directory, is created once, is never replaced, is not inside
//! `pos.db` (so not in the backups) and not in the nightly export. The
//! raw token never reaches the database, the logs or an event.
//!
//! The file handling follows `pos_export::pseudonym::load_or_create_key`
//! (written beside its name, linked into place, mode 0600).

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use rusqlite::Connection;
use sha2::Sha256;

use pos_db::queries::badges::{self, RevokeReason};

/// Name of the key file inside the data directory.
pub const KEY_FILE: &str = "badge.key";
const KEY_LEN: usize = 32;
/// Domain tag of the badge hash.
const HASH_DOMAIN: &[u8] = b"pili-badge/1\n";
/// What the key check signs.
const CHECK_MESSAGE: &[u8] = b"pili-badge-check/1";

type HmacSha256 = Hmac<Sha256>;

fn mac(key: &[u8; KEY_LEN]) -> HmacSha256 {
    // HMAC accepts a key of any length.
    HmacSha256::new_from_slice(key).expect("HMAC takes any key length")
}

/// The keyed hash of a normalized badge token: 64 lowercase hex
/// characters.
pub fn hash_token(key: &[u8; KEY_LEN], token: &str) -> String {
    let mut m = mac(key);
    m.update(HASH_DOMAIN);
    m.update(token.as_bytes());
    hex::encode(m.finalize().into_bytes())
}

/// The first 16 hex characters of `HMAC(key, "pili-badge-check/1")`:
/// stored in the database to notice another key later.
pub fn key_check(key: &[u8; KEY_LEN]) -> String {
    let mut m = mac(key);
    m.update(CHECK_MESSAGE);
    hex::encode(m.finalize().into_bytes())[..16].to_string()
}

/// What logs and the scan journal carry instead of the badge: the first
/// 12 characters of its hash.
pub fn badge_ref(hash: &str) -> &str {
    &hash[..12.min(hash.len())]
}

/// Read `data_dir/badge.key`, or create it with 32 bytes from the
/// operating system's randomness. A key that exists is never replaced; a
/// file of another size is an error and is left alone.
pub fn load_or_create(data_dir: &Path) -> Result<[u8; KEY_LEN], String> {
    load_or_create_with(data_dir, |from, to| fs::hard_link(from, to))
}

/// `load_or_create` with the link step injectable, so a test can force
/// the rename fallback of a file system without hard links.
fn load_or_create_with(
    data_dir: &Path,
    link: fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<[u8; KEY_LEN], String> {
    let path = data_dir.join(KEY_FILE);
    if let Some(key) = read_key(&path)? {
        return Ok(key);
    }
    fs::create_dir_all(data_dir).map_err(|e| format!("badge key: cannot create directory: {e}"))?;
    let mut key = [0u8; KEY_LEN];
    getrandom::fill(&mut key).map_err(|e| format!("badge key: no randomness: {e}"))?;
    let tmp = temp_path(data_dir);
    write_private(&tmp, &key)?;
    let linked = link(&tmp, &path);
    let _ = fs::remove_file(&tmp);
    match linked {
        Ok(()) => Ok(key),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            read_key(&path)?.ok_or_else(|| "badge key: file vanished while being read".to_string())
        }
        // A file system without hard links: a rename is still atomic.
        Err(_) => {
            let tmp = temp_path(data_dir);
            write_private(&tmp, &key)?;
            if path.exists() {
                let _ = fs::remove_file(&tmp);
                return read_key(&path)?
                    .ok_or_else(|| "badge key: file vanished while being read".to_string());
            }
            fs::rename(&tmp, &path).map_err(|e| {
                let _ = fs::remove_file(&tmp);
                format!("badge key: cannot store the key: {e}")
            })?;
            Ok(key)
        }
    }
}

/// `Some(key)` for an existing well-formed file, `None` when there is no
/// file, an error for a file of another size or one that cannot be read.
fn read_key(path: &Path) -> Result<Option<[u8; KEY_LEN]>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let key: [u8; KEY_LEN] = bytes.as_slice().try_into().map_err(|_| {
                format!(
                    "badge key: {} holds {} bytes, {KEY_LEN} expected; refusing to replace it",
                    path.display(),
                    bytes.len()
                )
            })?;
            tighten(path);
            Ok(Some(key))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("badge key: cannot read {}: {e}", path.display())),
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
        .map_err(|e| format!("badge key: cannot create {}: {e}", path.display()))?;
    let written = file.write_all(key).and_then(|()| file.sync_all());
    drop(file);
    written.map_err(|e| {
        let _ = fs::remove_file(path);
        format!("badge key: cannot write {}: {e}", path.display())
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

/// The lost-key check (L5-D4). When a key check is stored and the key
/// file is missing (a restore on new hardware) or gives another check,
/// every active badge is revoked with reason `key_changed` (one log row
/// each, no actor), a key is created if needed and the new check stored,
/// in one transaction. Returns how many badges were revoked.
///
/// A key file that exists but cannot be read, or has the wrong size, is
/// NOT a lost key: the error is returned and nothing is revoked.
pub fn open_check(conn: &Connection, data_dir: &Path) -> Result<usize, String> {
    let stored = badges::key_check(conn).map_err(|e| e.to_string())?;
    let Some(stored) = stored else {
        return Ok(0);
    };
    let path = data_dir.join(KEY_FILE);
    let key = match read_key(&path)? {
        Some(key) if key_check(&key) == stored => return Ok(0),
        Some(key) => key,
        None => load_or_create(data_dir)?,
    };
    badges::atomic(conn, |tx| {
        let revoked = badges::revoke_all_active(tx, RevokeReason::KeyChanged, None)?;
        badges::set_key_check(tx, &key_check(&key))?;
        Ok(revoked)
    })
    .map_err(|e| e.to_string())
}

/// The key to hash a scan or a link with: runs the lost-key check
/// first (a key file removed while the engine runs is noticed the same
/// way), then reads or creates the key.
pub fn key_for_use(conn: &Connection, data_dir: &Path) -> Result<[u8; KEY_LEN], String> {
    open_check(conn, data_dir)?;
    load_or_create(data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO users (insz, name, role) VALUES ('11111111111', 'M', 'admin');
             INSERT INTO users (insz, name, role) VALUES ('22222222222', 'A', 'waiter');
             INSERT INTO users (insz, name, role) VALUES ('33333333333', 'B', 'waiter');",
        )
        .unwrap();
        conn
    }

    fn fixed_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        key
    }

    #[test]
    fn the_hash_matches_a_known_hmac_vector() {
        // Python: hmac.new(bytes(range(32)), b"pili-badge/1\n04A1B2C3", sha256).hexdigest()
        assert_eq!(
            hash_token(&fixed_key(), "04A1B2C3"),
            "50215cea6712cef3193f574b4f718a95056a59ef2636ef2dc2ba96a5c7022c3f"
        );
        assert_eq!(key_check(&fixed_key()), "4283644baa79b963");
    }

    #[test]
    fn the_hash_depends_on_the_key_and_the_token() {
        let other = [7u8; 32];
        let a = hash_token(&fixed_key(), "04A1B2C3");
        assert_eq!(a.len(), 64);
        assert_ne!(a, hash_token(&other, "04A1B2C3"));
        assert_ne!(a, hash_token(&fixed_key(), "04A1B2C4"));
        assert_eq!(a, hash_token(&fixed_key(), "04A1B2C3"));
    }

    #[test]
    fn the_key_check_detects_another_key() {
        assert_ne!(key_check(&fixed_key()), key_check(&[9u8; 32]));
        assert_eq!(key_check(&fixed_key()).len(), 16);
    }

    #[test]
    fn the_badge_ref_is_twelve_characters_of_the_hash() {
        let h = hash_token(&fixed_key(), "04A1B2C3");
        assert_eq!(badge_ref(&h), &h[..12]);
        assert_eq!(badge_ref("abc"), "abc");
    }

    #[test]
    fn the_key_is_created_once_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create(dir.path()).unwrap();
        let again = load_or_create(dir.path()).unwrap();
        assert_eq!(first, again);
        assert_eq!(fs::read(dir.path().join(KEY_FILE)).unwrap(), first);
        assert_ne!(first, [0u8; 32]);
        // No temporary file is left beside it.
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![KEY_FILE.to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn the_key_file_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        load_or_create(dir.path()).unwrap();
        let path = dir.path().join(KEY_FILE);
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // An existing key with a looser mode is tightened on read.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        load_or_create(dir.path()).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn a_wrong_size_file_is_an_error_and_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE);
        fs::write(&path, b"short").unwrap();
        let err = load_or_create(dir.path()).unwrap_err();
        assert!(err.contains("5 bytes"), "{err}");
        assert_eq!(fs::read(&path).unwrap(), b"short");
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n != KEY_FILE)
            .collect()
    }

    #[test]
    fn concurrent_creators_all_get_the_winners_key() {
        let dir = tempfile::tempdir().unwrap();
        let barrier = std::sync::Barrier::new(8);
        let keys: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        barrier.wait();
                        load_or_create(dir.path())
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let first = *keys[0].as_ref().expect("the first creator got a key");
        for key in &keys {
            assert_eq!(key.as_ref().expect("every creator succeeds"), &first);
        }
        assert_eq!(fs::read(dir.path().join(KEY_FILE)).unwrap(), first);
        assert!(
            leftovers(dir.path()).is_empty(),
            "{:?}",
            leftovers(dir.path())
        );
    }

    fn no_hard_links(_: &Path, _: &Path) -> std::io::Result<()> {
        Err(std::io::Error::from(ErrorKind::Unsupported))
    }

    #[test]
    fn without_hard_links_the_key_is_stored_by_rename() {
        let dir = tempfile::tempdir().unwrap();
        let key = load_or_create_with(dir.path(), no_hard_links).unwrap();
        assert_eq!(fs::read(dir.path().join(KEY_FILE)).unwrap(), key);
        assert!(
            leftovers(dir.path()).is_empty(),
            "{:?}",
            leftovers(dir.path())
        );
        assert_eq!(load_or_create(dir.path()).unwrap(), key);
    }

    #[test]
    fn the_rename_fallback_never_overwrites_an_existing_key() {
        // The key appears between the first read and the link step: the
        // link refuses (as a racing creator's would not), the fallback
        // must read the winner's key, not replace it.
        fn winner_appears(_: &Path, target: &Path) -> std::io::Result<()> {
            fs::write(target, [9u8; KEY_LEN])?;
            Err(std::io::Error::from(ErrorKind::Unsupported))
        }
        let dir = tempfile::tempdir().unwrap();
        let key = load_or_create_with(dir.path(), winner_appears).unwrap();
        assert_eq!(key, [9u8; KEY_LEN]);
        assert_eq!(fs::read(dir.path().join(KEY_FILE)).unwrap(), [9u8; KEY_LEN]);
        assert!(
            leftovers(dir.path()).is_empty(),
            "{:?}",
            leftovers(dir.path())
        );
    }

    #[test]
    fn the_directory_is_created_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        load_or_create(&nested).unwrap();
        assert!(nested.join(KEY_FILE).is_file());
    }

    #[test]
    fn without_a_stored_check_nothing_happens() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(open_check(&conn, dir.path()).unwrap(), 0);
        assert!(!dir.path().join(KEY_FILE).exists(), "no key is created");
    }

    #[test]
    fn a_matching_key_revokes_nothing() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        let key = load_or_create(dir.path()).unwrap();
        badges::set_key_check(&conn, &key_check(&key)).unwrap();
        badges::link(
            &conn,
            &hash_token(&key, "04A1B2C3"),
            "22222222222",
            "11111111111",
        )
        .unwrap();
        assert_eq!(open_check(&conn, dir.path()).unwrap(), 0);
        assert_eq!(badges::list_active(&conn).unwrap().len(), 1);
    }

    #[test]
    fn a_missing_key_file_revokes_every_badge_and_stores_the_new_check() {
        let conn = db();
        let old_dir = tempfile::tempdir().unwrap();
        let old = load_or_create(old_dir.path()).unwrap();
        badges::set_key_check(&conn, &key_check(&old)).unwrap();
        badges::link(
            &conn,
            &hash_token(&old, "04A1B2C3"),
            "22222222222",
            "11111111111",
        )
        .unwrap();
        badges::link(
            &conn,
            &hash_token(&old, "04A1B2C4"),
            "33333333333",
            "11111111111",
        )
        .unwrap();

        // The database restored on new hardware: no key file there.
        let new_dir = tempfile::tempdir().unwrap();
        assert_eq!(open_check(&conn, new_dir.path()).unwrap(), 2);
        assert!(badges::list_active(&conn).unwrap().is_empty());
        let key = fs::read(new_dir.path().join(KEY_FILE)).unwrap();
        assert_eq!(key.len(), 32);
        let new: [u8; 32] = key.try_into().unwrap();
        assert_eq!(badges::key_check(&conn).unwrap(), Some(key_check(&new)));
        assert!(badges::relink_needed(&conn).unwrap());
        // Run again: nothing more to revoke.
        assert_eq!(open_check(&conn, new_dir.path()).unwrap(), 0);
    }

    #[test]
    fn another_key_file_revokes_every_badge() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        badges::set_key_check(&conn, &key_check(&fixed_key())).unwrap();
        badges::link(
            &conn,
            &hash_token(&fixed_key(), "04A1B2C3"),
            "22222222222",
            "11111111111",
        )
        .unwrap();
        let other = load_or_create(dir.path()).unwrap();
        assert_eq!(open_check(&conn, dir.path()).unwrap(), 1);
        assert_eq!(badges::key_check(&conn).unwrap(), Some(key_check(&other)));
    }

    #[test]
    fn an_unreadable_key_revokes_nothing() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        badges::set_key_check(&conn, &key_check(&fixed_key())).unwrap();
        badges::link(
            &conn,
            &hash_token(&fixed_key(), "04A1B2C3"),
            "22222222222",
            "11111111111",
        )
        .unwrap();
        fs::write(dir.path().join(KEY_FILE), b"wrong size").unwrap();
        assert!(open_check(&conn, dir.path()).is_err());
        assert!(key_for_use(&conn, dir.path()).is_err());
        assert_eq!(badges::list_active(&conn).unwrap().len(), 1);
    }
}
