//! The FDM's certificate is trusted on its CA's signature and its
//! dates (`pos_fiscal::fdm_tls`), with module2's real certificates:
//! `fixtures/module2_tls/ca.crt` (manual.module2.be, "Download losse
//! certificaten") and `server.crt`, the X.509 v1 certificate without
//! any name that fdm.module2.be:443 served on 2026-09-27.

use pos_fiscal::fdm_tls::{SignedByCa, client_config};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::{CertificateError, Error};
use std::time::Duration;

fn der(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/module2_tls/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    CertificateDer::from_pem_file(path).unwrap().to_vec()
}

fn at_year(year: i32) -> UnixTime {
    let secs = chrono::NaiveDate::from_ymd_opt(year, 6, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    UnixTime::since_unix_epoch(Duration::from_secs(secs as u64))
}

fn module2() -> SignedByCa {
    SignedByCa::new(der("ca.crt")).unwrap()
}

#[test]
fn the_module2_server_certificate_is_trusted() {
    module2()
        .check(&der("server.crt"), UnixTime::now())
        .unwrap();
}

#[test]
fn a_certificate_the_ca_did_not_sign_is_refused() {
    // Same subject and issuer names, one byte of the signature changed.
    let mut forged = der("server.crt");
    let last = forged.len() - 1;
    forged[last] ^= 0x01;
    assert_eq!(
        module2().check(&forged, UnixTime::now()),
        Err(Error::InvalidCertificate(CertificateError::BadSignature))
    );
}

#[test]
fn another_ca_is_not_the_issuer() {
    let other = SignedByCa::new(der("server.crt")).unwrap();
    assert_eq!(
        other.check(&der("server.crt"), UnixTime::now()),
        Err(Error::InvalidCertificate(CertificateError::UnknownIssuer))
    );
}

#[test]
fn the_certificate_dates_are_checked() {
    assert_eq!(
        module2().check(&der("server.crt"), at_year(2020)),
        Err(Error::InvalidCertificate(CertificateError::NotValidYet))
    );
    assert_eq!(
        module2().check(&der("server.crt"), at_year(3001)),
        Err(Error::InvalidCertificate(CertificateError::Expired))
    );
}

#[test]
fn a_missing_file_is_named() {
    let err = client_config("/nonexistent/ca.crt", None).unwrap_err();
    assert!(err.to_string().contains("/nonexistent/ca.crt"), "{err}");
    let ca = format!(
        "{}/tests/fixtures/module2_tls/ca.crt",
        env!("CARGO_MANIFEST_DIR")
    );
    client_config(&ca, None).expect("the CA alone is enough for HTTPS without mTLS");
}

#[test]
fn an_unusable_https_setup_is_a_config_problem() {
    let config = pos_core::config::PosConfig {
        fdm_url: "https://fdm.module2.be/graphql/".into(),
        fdm_ca_file: Some(format!(
            "{}/tests/fixtures/module2_tls/server.crt",
            env!("CARGO_MANIFEST_DIR")
        )),
        fdm_client_cert_file: Some("/nonexistent/client.crt".into()),
        fdm_client_key_file: Some("/nonexistent/client.key".into()),
        ..Default::default()
    };
    let problems = pos_fiscal::fdm_client::FdmClient::config_problems(&config);
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with("FDM HTTPS:") && p.contains("/nonexistent/client.crt")),
        "{problems:?}"
    );
}

#[test]
fn an_expired_ca_does_not_vouch_for_a_valid_certificate() {
    // ca.crt ends 3000-01-03 23:33:23 GMT, server.crt 31 seconds later:
    // in between, the server certificate is within its dates but its CA
    // is not.
    let between = UnixTime::since_unix_epoch(Duration::from_secs(
        chrono::NaiveDate::from_ymd_opt(3000, 1, 3)
            .unwrap()
            .and_hms_opt(23, 33, 40)
            .unwrap()
            .and_utc()
            .timestamp()
            .try_into()
            .unwrap(),
    ));
    assert_eq!(
        module2().check(&der("server.crt"), between),
        Err(Error::InvalidCertificate(CertificateError::Expired))
    );
}

#[test]
fn every_ca_of_a_bundle_counts() {
    // The right CA second, an unrelated certificate first.
    let bundle = SignedByCa::from_cas(vec![der("server.crt"), der("ca.crt")]).unwrap();
    bundle.check(&der("server.crt"), UnixTime::now()).unwrap();
    assert!(SignedByCa::from_cas(Vec::new()).is_none());

    // And from a PEM file holding two certificates.
    let dir = std::env::temp_dir().join(format!("fdm-tls-bundle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fixtures = format!("{}/tests/fixtures/module2_tls", env!("CARGO_MANIFEST_DIR"));
    let mut pem = std::fs::read_to_string(format!("{fixtures}/server.crt")).unwrap();
    pem.push_str(&std::fs::read_to_string(format!("{fixtures}/ca.crt")).unwrap());
    let path = dir.join("bundle.pem");
    std::fs::write(&path, pem).unwrap();
    client_config(path.to_str().unwrap(), None).expect("a bundle of two certificates");
    std::fs::remove_dir_all(&dir).ok();
}
