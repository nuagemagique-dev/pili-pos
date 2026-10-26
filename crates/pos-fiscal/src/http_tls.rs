//! How the engine's HTTPS clients decide whom to trust.
//!
//! The host picks the mode when it opens the engine; the engine never
//! infers it from the operating system. This is the one place that
//! touches reqwest's TLS builder options for the clients built from an
//! [`HttpTls`] (the `HubRise` client, the digital-ticket client and the
//! FDM client without mTLS files). The FDM's own CA and client
//! certificate path ([`crate::fdm_tls`]) is separate and unchanged.

/// Whom the engine's HTTPS clients trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpTls {
    /// reqwest's default client: the platform verifier of the build
    /// (the system store on Linux). No `tls_*` option is set; this is
    /// what the Linux shell passes.
    Platform,
    /// Trust exactly these root certificates (DER), nothing else. An
    /// empty list is refused ([`HttpTlsError::NoRoots`]).
    RootsOnly(Vec<Vec<u8>>),
}

/// An [`HttpTls`] that cannot make a client.
#[derive(Debug, thiserror::Error)]
pub enum HttpTlsError {
    #[error("no trusted root certificate: an HTTPS client would trust nothing")]
    NoRoots,
    #[error("invalid root certificate (DER): {0}")]
    InvalidCertificate(#[from] reqwest::Error),
}

impl HttpTls {
    /// A client builder with this trust. `Platform` is
    /// `reqwest::Client::builder()` untouched.
    ///
    /// reqwest reads the certificates when the client is built, so an
    /// invalid DER shows at `build()`, not here; [`HttpTls::check`]
    /// does both.
    pub fn client_builder(&self) -> Result<reqwest::ClientBuilder, HttpTlsError> {
        let builder = reqwest::Client::builder();
        match self {
            HttpTls::Platform => Ok(builder),
            HttpTls::RootsOnly(ders) if ders.is_empty() => Err(HttpTlsError::NoRoots),
            HttpTls::RootsOnly(ders) => {
                let certs = ders
                    .iter()
                    .map(|der| reqwest::Certificate::from_der(der))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(builder.tls_certs_only(certs))
            }
        }
    }

    /// Whether a client can be built with this trust: no roots and
    /// roots that are not certificates are refused. The engine calls
    /// this when it opens, so the clients it builds later cannot fail.
    pub fn check(&self) -> Result<(), HttpTlsError> {
        self.client_builder()?.build()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mock FDM's CA (a public test certificate), PEM to DER.
    fn mock_ca_der() -> Vec<u8> {
        use base64::Engine as _;
        let pem = include_str!("../../pos-fdm-mock/fixtures/tls/ca.crt");
        let body: String = pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<Vec<_>>()
            .join("");
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .expect("ca.crt is base64 PEM")
    }

    #[test]
    fn platform_builds() {
        HttpTls::Platform.client_builder().unwrap().build().unwrap();
        HttpTls::Platform.check().unwrap();
    }

    #[test]
    fn no_roots_is_refused() {
        let err = HttpTls::RootsOnly(vec![]).client_builder().unwrap_err();
        assert!(matches!(err, HttpTlsError::NoRoots), "{err}");
    }

    #[test]
    fn an_invalid_der_is_an_error() {
        let tls = HttpTls::RootsOnly(vec![b"not a certificate".to_vec()]);
        // reqwest reads the certificates when it builds the client.
        let built = tls.client_builder().and_then(|b| Ok(b.build()?));
        assert!(built.is_err());
        let err = tls.check().unwrap_err();
        assert!(matches!(err, HttpTlsError::InvalidCertificate(_)), "{err}");
    }

    #[test]
    fn a_valid_der_root_builds() {
        let ders = vec![mock_ca_der()];
        HttpTls::RootsOnly(ders)
            .client_builder()
            .unwrap()
            .build()
            .unwrap();
        HttpTls::RootsOnly(vec![mock_ca_der()]).check().unwrap();
    }
}
