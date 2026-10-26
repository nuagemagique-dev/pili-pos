//! age encryption of the backup (X25519 recipients, streaming).

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use age::secrecy::ExposeSecret;

/// `<path>.partial`: where a file is written before it is renamed.
fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".partial");
    PathBuf::from(name)
}

/// Encrypt `plain` to every recipient and write the result to `out`; return
/// the size of `out`. The data goes through a stream, never whole in memory.
/// The file is written to `<out>.partial`, synced and renamed, so `out` is
/// either absent (or the previous one) or complete.
pub fn encrypt_file(
    plain: &Path,
    out: &Path,
    recipients: &[age::x25519::Recipient],
) -> Result<i64, String> {
    if recipients.is_empty() {
        return Err("encrypt: no recipient".to_string());
    }
    let partial = partial_path(out);
    let result = encrypt_to(plain, &partial, recipients).and_then(|()| {
        std::fs::rename(&partial, out)
            .map_err(|e| format!("encrypt: cannot rename {}: {e}", partial.display()))
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result?;
    let len = std::fs::metadata(out)
        .map_err(|e| format!("encrypt: cannot stat {}: {e}", out.display()))?
        .len();
    i64::try_from(len).map_err(|_| "encrypt: size out of range".to_string())
}

fn encrypt_to(
    plain: &Path,
    partial: &Path,
    recipients: &[age::x25519::Recipient],
) -> Result<(), String> {
    let mut input = BufReader::new(
        File::open(plain).map_err(|e| format!("encrypt: cannot open {}: {e}", plain.display()))?,
    );
    let file = File::create(partial)
        .map_err(|e| format!("encrypt: cannot create {}: {e}", partial.display()))?;
    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r as &dyn age::Recipient))
            .map_err(|e| format!("encrypt: {e}"))?;
    let mut writer = encryptor
        .wrap_output(BufWriter::new(file))
        .map_err(|e| format!("encrypt: {e}"))?;
    std::io::copy(&mut input, &mut writer).map_err(|e| format!("encrypt: write: {e}"))?;
    let buffered = writer
        .finish()
        .map_err(|e| format!("encrypt: finish: {e}"))?;
    let file = buffered
        .into_inner()
        .map_err(|e| format!("encrypt: flush: {}", e.error()))?;
    file.sync_all().map_err(|e| format!("encrypt: sync: {e}"))
}

/// Decrypt the age file `input` with the secret key `identity`
/// (`AGE-SECRET-KEY-1...`) into `out`. For tests and the recovery
/// instructions; the POS itself never holds a secret key. The error never
/// holds the key.
pub fn decrypt_file(identity: &str, input: &Path, out: &Path) -> Result<(), String> {
    // Deliberately drops the parser error: it can echo part of the secret key.
    let identity = age::x25519::Identity::from_str(identity.trim())
        .map_err(|_| "decrypt: not an age secret key".to_string())?;
    let file =
        File::open(input).map_err(|e| format!("decrypt: cannot open {}: {e}", input.display()))?;
    let decryptor =
        age::Decryptor::new(BufReader::new(file)).map_err(|e| format!("decrypt: {e}"))?;
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| format!("decrypt: {e}"))?;
    let partial = partial_path(out);
    let result = (|| {
        let mut w = BufWriter::new(
            File::create(&partial)
                .map_err(|e| format!("decrypt: cannot create {}: {e}", partial.display()))?,
        );
        std::io::copy(&mut reader, &mut w).map_err(|e| format!("decrypt: {e}"))?;
        w.flush().map_err(|e| format!("decrypt: flush: {e}"))?;
        std::fs::rename(&partial, out)
            .map_err(|e| format!("decrypt: cannot rename {}: {e}", partial.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

/// A fresh X25519 identity as `(secret, public)`: the secret is
/// `AGE-SECRET-KEY-1...`, the public `age1...`. For tests and tools; the
/// POS never generates a key.
pub fn generate_identity() -> (String, String) {
    let id = age::x25519::Identity::generate();
    (
        id.to_string().expose_secret().to_string(),
        id.to_public().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipient(public: &str) -> age::x25519::Recipient {
        age::x25519::Recipient::from_str(public).unwrap()
    }

    fn payload() -> Vec<u8> {
        // Larger than one age chunk (64 KiB), with a recognisable marker.
        let mut v = b"SQLite format 3\0-MARKER-pizza-margherita-".to_vec();
        for i in 0..200_000u32 {
            v.push((i % 251) as u8);
        }
        v
    }

    #[test]
    fn owner_and_producer_both_decrypt_to_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let (owner_sk, owner_pk) = generate_identity();
        let (prod_sk, prod_pk) = generate_identity();
        let plain = dir.path().join("pos.db");
        let data = payload();
        std::fs::write(&plain, &data).unwrap();
        let out = dir.path().join("pos.db.age");

        let size =
            encrypt_file(&plain, &out, &[recipient(&owner_pk), recipient(&prod_pk)]).unwrap();
        assert_eq!(size, std::fs::metadata(&out).unwrap().len() as i64);
        assert!(size > data.len() as i64);
        assert!(!dir.path().join("pos.db.age.partial").exists());

        for (name, sk) in [("owner", &owner_sk), ("producer", &prod_sk)] {
            let back = dir.path().join(format!("{name}.db"));
            decrypt_file(sk, &out, &back).unwrap();
            assert_eq!(std::fs::read(&back).unwrap(), data, "{name}");
        }
    }

    #[test]
    fn a_third_identity_cannot_decrypt() {
        let dir = tempfile::tempdir().unwrap();
        let (_, owner_pk) = generate_identity();
        let (third_sk, _) = generate_identity();
        let plain = dir.path().join("p");
        std::fs::write(&plain, payload()).unwrap();
        let out = dir.path().join("p.age");
        encrypt_file(&plain, &out, &[recipient(&owner_pk)]).unwrap();
        let back = dir.path().join("back");
        let err = decrypt_file(&third_sk, &out, &back).unwrap_err();
        assert!(err.starts_with("decrypt: "), "{err}");
        assert!(!err.contains(&third_sk));
        assert!(!back.exists());
        assert!(!dir.path().join("back.partial").exists());
    }

    #[test]
    fn the_output_is_an_age_file_without_the_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let (_, pk) = generate_identity();
        let plain = dir.path().join("p");
        std::fs::write(&plain, payload()).unwrap();
        let out = dir.path().join("p.age");
        encrypt_file(&plain, &out, &[recipient(&pk)]).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"age-encryption.org/v1"));
        let hay = String::from_utf8_lossy(&bytes);
        assert!(!hay.contains("MARKER-pizza-margherita"));
        assert!(!hay.contains("SQLite format 3"));
    }

    #[test]
    fn an_empty_recipient_list_is_an_error_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("p");
        std::fs::write(&plain, b"x").unwrap();
        let out = dir.path().join("p.age");
        assert!(encrypt_file(&plain, &out, &[]).is_err());
        assert!(!out.exists());
        assert!(!dir.path().join("p.age.partial").exists());
    }

    #[test]
    fn a_missing_plaintext_leaves_no_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let (_, pk) = generate_identity();
        let out = dir.path().join("p.age");
        assert!(encrypt_file(&dir.path().join("absent"), &out, &[recipient(&pk)]).is_err());
        assert!(!out.exists());
        assert!(!dir.path().join("p.age.partial").exists());
    }

    #[test]
    fn an_empty_file_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let (sk, pk) = generate_identity();
        let plain = dir.path().join("p");
        std::fs::write(&plain, b"").unwrap();
        let out = dir.path().join("p.age");
        encrypt_file(&plain, &out, &[recipient(&pk)]).unwrap();
        let back = dir.path().join("back");
        decrypt_file(&sk, &out, &back).unwrap();
        assert!(std::fs::read(&back).unwrap().is_empty());
    }

    #[test]
    fn a_garbage_key_is_refused_without_echoing_it() {
        let dir = tempfile::tempdir().unwrap();
        let err = decrypt_file(
            "not-a-key-xyz",
            &dir.path().join("a"),
            &dir.path().join("b"),
        )
        .unwrap_err();
        assert!(!err.contains("not-a-key-xyz"));
    }

    #[test]
    fn generated_identities_have_the_expected_shape() {
        let (sk, pk) = generate_identity();
        assert!(sk.starts_with("AGE-SECRET-KEY-1"));
        assert!(pk.starts_with("age1") && pk.len() == 62);
        let (sk2, _) = generate_identity();
        assert_ne!(sk, sk2);
    }
}
