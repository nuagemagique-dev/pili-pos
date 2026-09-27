//! HTTPS to the FDM, with the certificates FDM vendors hand out.
//!
//! module2 (FDM software 2.2.0.5, its manual "HTTPS en mTLS") serves
//! every FDM and its online simulator with an X.509 v1 server
//! certificate signed by its own root CA, without any host name, and
//! asks the POS for a client certificate (mTLS). The usual checks
//! (webpki) refuse a v1 certificate and need a host name, and a LAN
//! FDM is reached by IP anyway. So the FDM is trusted on what can be
//! checked: its certificate is signed by the configured CA and within
//! its dates, and the handshake is signed with that certificate's key.
//! The CA stands for the name check: a certificate from another CA, or
//! a stolen certificate without its key, is refused.

use std::sync::Arc;

use aws_lc_rs::signature::{self, UnparsedPublicKey, VerificationAlgorithm};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, Error, SignatureScheme};
use x509_parser::prelude::{FromDer, X509Certificate};

/// Why a TLS setup could not be built from the configured files.
#[derive(Debug, thiserror::Error)]
pub enum TlsSetupError {
    #[error("cannot read {0:?}: {1}")]
    Read(String, std::io::Error),
    #[error("{0:?} holds no PEM {1}")]
    Pem(String, &'static str),
    #[error("{0:?} is not an X.509 certificate")]
    Certificate(String),
    #[error("TLS setup refused: {0}")]
    Rustls(#[from] Error),
}

fn read(path: &str) -> Result<Vec<u8>, TlsSetupError> {
    std::fs::read(path).map_err(|e| TlsSetupError::Read(path.to_string(), e))
}

/// A rustls client for an FDM whose server certificate is signed by the
/// CA in `ca_file`, presenting the client certificate of `client` (cert
/// and key files, PEM) when set.
pub fn client_config(
    ca_file: &str,
    client: Option<(&str, &str)>,
) -> Result<rustls::ClientConfig, TlsSetupError> {
    let ca = CertificateDer::from_pem_slice(&read(ca_file)?)
        .map_err(|_| TlsSetupError::Pem(ca_file.to_string(), "certificate"))?;
    let verifier = SignedByCa::new(ca.to_vec())
        .ok_or_else(|| TlsSetupError::Certificate(ca_file.to_string()))?;
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier));
    let config = match client {
        Some((cert_file, key_file)) => {
            let cert = CertificateDer::from_pem_slice(&read(cert_file)?)
                .map_err(|_| TlsSetupError::Pem(cert_file.to_string(), "certificate"))?;
            let key = PrivateKeyDer::from_pem_slice(&read(key_file)?)
                .map_err(|_| TlsSetupError::Pem(key_file.to_string(), "private key"))?;
            // Not `with_client_auth_cert`: it matches the key to the
            // certificate through webpki, which refuses module2's X.509
            // v1 client certificate. The FDM checks the pair anyway.
            let signing_key = provider.key_provider.load_private_key(key)?;
            let certified = rustls::sign::CertifiedKey::new(vec![cert], signing_key);
            builder.with_client_cert_resolver(Arc::new(OneClientCert(Arc::new(certified))))
        }
        None => builder.with_no_client_auth(),
    };
    Ok(config)
}

/// The one client certificate the POS presents to the FDM.
#[derive(Debug)]
struct OneClientCert(Arc<rustls::sign::CertifiedKey>);

impl rustls::client::ResolvesClientCert for OneClientCert {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[SignatureScheme],
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        Some(self.0.clone())
    }

    fn has_certs(&self) -> bool {
        true
    }
}

/// Trusts a server certificate signed by one CA, whatever its version
/// and names.
#[derive(Debug)]
pub struct SignedByCa {
    ca_der: Vec<u8>,
}

impl SignedByCa {
    /// `None` when `ca_der` is not a certificate.
    pub fn new(ca_der: Vec<u8>) -> Option<Self> {
        X509Certificate::from_der(&ca_der).ok()?;
        Some(Self { ca_der })
    }

    /// The end-entity certificate is signed by the CA and valid at `now`.
    pub fn check(&self, end_entity: &[u8], now: UnixTime) -> Result<(), Error> {
        let bad = |e: CertificateError| Error::InvalidCertificate(e);
        let (_, cert) = X509Certificate::from_der(end_entity)
            .map_err(|_| bad(CertificateError::BadEncoding))?;
        let (_, ca) = X509Certificate::from_der(&self.ca_der)
            .map_err(|_| bad(CertificateError::BadEncoding))?;
        if cert.issuer().as_raw() != ca.subject().as_raw() {
            return Err(bad(CertificateError::UnknownIssuer));
        }
        let algorithm = certificate_signature_algorithm(&cert.signature_algorithm.algorithm)
            .ok_or_else(|| bad(CertificateError::BadSignature))?;
        UnparsedPublicKey::new(algorithm, ca.public_key().subject_public_key.data.as_ref())
            .verify(
                cert.tbs_certificate.as_ref(),
                cert.signature_value.data.as_ref(),
            )
            .map_err(|_| bad(CertificateError::BadSignature))?;
        let now = now.as_secs() as i64;
        let validity = cert.validity();
        if now < validity.not_before.timestamp() {
            return Err(bad(CertificateError::NotValidYet));
        }
        if now > validity.not_after.timestamp() {
            return Err(bad(CertificateError::Expired));
        }
        Ok(())
    }
}

/// The verification algorithm of a certificate signature, by its OID.
fn certificate_signature_algorithm(
    oid: &x509_parser::der_parser::oid::Oid<'_>,
) -> Option<&'static dyn VerificationAlgorithm> {
    Some(match oid.to_id_string().as_str() {
        "1.2.840.113549.1.1.11" => &signature::RSA_PKCS1_2048_8192_SHA256,
        "1.2.840.113549.1.1.12" => &signature::RSA_PKCS1_2048_8192_SHA384,
        "1.2.840.113549.1.1.13" => &signature::RSA_PKCS1_2048_8192_SHA512,
        "1.2.840.10045.4.3.2" => &signature::ECDSA_P256_SHA256_ASN1,
        "1.2.840.10045.4.3.3" => &signature::ECDSA_P384_SHA384_ASN1,
        _ => return None,
    })
}

/// The verification algorithm of a TLS handshake signature.
fn handshake_signature_algorithm(
    scheme: SignatureScheme,
) -> Option<&'static dyn VerificationAlgorithm> {
    Some(match scheme {
        SignatureScheme::RSA_PSS_SHA256 => &signature::RSA_PSS_2048_8192_SHA256,
        SignatureScheme::RSA_PSS_SHA384 => &signature::RSA_PSS_2048_8192_SHA384,
        SignatureScheme::RSA_PSS_SHA512 => &signature::RSA_PSS_2048_8192_SHA512,
        SignatureScheme::RSA_PKCS1_SHA256 => &signature::RSA_PKCS1_2048_8192_SHA256,
        SignatureScheme::RSA_PKCS1_SHA384 => &signature::RSA_PKCS1_2048_8192_SHA384,
        SignatureScheme::RSA_PKCS1_SHA512 => &signature::RSA_PKCS1_2048_8192_SHA512,
        SignatureScheme::ECDSA_NISTP256_SHA256 => &signature::ECDSA_P256_SHA256_ASN1,
        SignatureScheme::ECDSA_NISTP384_SHA384 => &signature::ECDSA_P384_SHA384_ASN1,
        _ => return None,
    })
}

/// The handshake `message` is signed with the key of `cert`.
fn check_handshake(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, Error> {
    let algorithm = handshake_signature_algorithm(dss.scheme).ok_or(Error::PeerIncompatible(
        rustls::PeerIncompatible::NoSignatureSchemesInCommon,
    ))?;
    let (_, cert) = X509Certificate::from_der(cert.as_ref())
        .map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))?;
    UnparsedPublicKey::new(
        algorithm,
        cert.public_key().subject_public_key.data.as_ref(),
    )
    .verify(message, dss.signature())
    .map_err(|_| Error::InvalidCertificate(CertificateError::BadSignature))?;
    Ok(HandshakeSignatureValid::assertion())
}

impl ServerCertVerifier for SignedByCa {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        self.check(end_entity.as_ref(), now)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        check_handshake(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // TLS 1.3 forbids RSA PKCS#1 v1.5 in the handshake.
        if matches!(
            dss.scheme,
            SignatureScheme::RSA_PKCS1_SHA256
                | SignatureScheme::RSA_PKCS1_SHA384
                | SignatureScheme::RSA_PKCS1_SHA512
        ) {
            return Err(Error::PeerMisbehaved(
                rustls::PeerMisbehaved::SignedHandshakeWithUnadvertisedSigScheme,
            ));
        }
        check_handshake(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
        ]
    }
}
