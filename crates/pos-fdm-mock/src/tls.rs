//! HTTPS like module2's FDMs and simulator (manual.module2.be, "HTTPS
//! en mTLS"): an X.509 v1 server certificate without any name, signed
//! by the FDM vendor's CA, and a client certificate asked for. module2's
//! simulator answers without one too; `require_client` makes it
//! mandatory, to prove the POS sends it. The client certificate is not
//! checked: this is a mock.

use std::path::Path;
use std::sync::Arc;

use rustls::client::danger::HandshakeSignatureValid;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::sign::CertifiedKey;
use rustls::{DigitallySignedStruct, DistinguishedName, Error, SignatureScheme};

/// The mock's server certificate, whatever its version.
#[derive(Debug)]
struct OneServerCert(Arc<CertifiedKey>);

impl rustls::server::ResolvesServerCert for OneServerCert {
    fn resolve(&self, _hello: rustls::server::ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}

/// Asks for a client certificate and takes whatever comes.
#[derive(Debug)]
struct AnyClientCert {
    mandatory: bool,
}

impl ClientCertVerifier for AnyClientCert {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        self.mandatory
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
        ]
    }
}

/// The TLS settings of a mock serving `dir/server.crt` and
/// `dir/server.key` (PEM).
pub fn server_config(dir: &Path, require_client: bool) -> std::io::Result<rustls::ServerConfig> {
    let invalid = |e: String| std::io::Error::new(std::io::ErrorKind::InvalidData, e);
    let cert = CertificateDer::from_pem_file(dir.join("server.crt"))
        .map_err(|e| invalid(format!("server.crt: {e}")))?;
    let key = PrivateKeyDer::from_pem_file(dir.join("server.key"))
        .map_err(|e| invalid(format!("server.key: {e}")))?;
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let signing_key = provider
        .key_provider
        .load_private_key(key)
        .map_err(|e| invalid(e.to_string()))?;
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| invalid(e.to_string()))?
        .with_client_cert_verifier(Arc::new(AnyClientCert {
            mandatory: require_client,
        }))
        .with_cert_resolver(Arc::new(OneServerCert(Arc::new(CertifiedKey::new(
            vec![cert],
            signing_key,
        )))));
    Ok(config)
}

/// Serve `router` over HTTPS on `listener` until the task is dropped.
pub async fn serve(
    listener: std::net::TcpListener,
    router: axum::Router,
    dir: &Path,
    require_client: bool,
) -> std::io::Result<()> {
    listener.set_nonblocking(true)?;
    let config = axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(server_config(
        dir,
        require_client,
    )?));
    axum_server::from_tcp_rustls(listener, config)
        .serve(router.into_make_service())
        .await
}
