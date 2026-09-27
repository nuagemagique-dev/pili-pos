//! The POS's HTTPS to the FDM, offline: the mock FDM serves TLS like
//! module2 (an X.509 v1 server certificate without names, signed by its
//! CA) and here requires the client certificate, so the handshake only
//! succeeds when the POS presents it. Certificates:
//! `crates/pos-fdm-mock/fixtures/tls` (test-only, v1 like module2's).

use pos_core::config::PosConfig;
use pos_fiscal::event::{Booking, build_work_in_out};
use pos_fiscal::fdm_client::FdmClient;

fn mock_tls(name: &str) -> String {
    format!(
        "{}/../pos-fdm-mock/fixtures/tls/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A mock FDM on HTTPS, client certificate required; its URL.
fn https_fdm() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let dir = std::path::PathBuf::from(mock_tls(""));
    tokio::spawn(async move {
        pos_fdm_mock::tls::serve(
            listener,
            pos_fdm_mock::app_with(Default::default()),
            &dir,
            true,
        )
        .await
        .unwrap();
    });
    format!("https://127.0.0.1:{port}/graphql")
}

fn config(url: &str, ca: String, client: bool) -> PosConfig {
    PosConfig {
        pos_id: "CPOS0031234567".into(),
        terminal_id: "POS-1-BAR".into(),
        device_id: "F0:1D:3D:CC:13:C8".into(),
        fdm_url: url.to_string(),
        fdm_ca_file: Some(ca),
        fdm_client_cert_file: client.then(|| mock_tls("client.crt")),
        fdm_client_key_file: client.then(|| mock_tls("client.key")),
        ..Default::default()
    }
}

#[tokio::test]
async fn the_pos_signs_over_mtls_with_v1_certificates() {
    let url = https_fdm();
    let config = config(&url, mock_tls("ca.crt"), true);
    assert!(
        FdmClient::config_problems(&config)
            .iter()
            .all(|p| !p.starts_with("FDM HTTPS"))
    );
    let fdm = FdmClient::from_config(&config);
    let status = fdm.status().await.expect("status over mTLS");
    assert_eq!(status.device.fdm_id, "MCK00000001");
    let booking = Booking {
        period_id: "8f1c2c1e-0000-4000-8000-000000000002".into(),
        date: pos_fiscal::event::fdm_now()[..10].to_string(),
    };
    let signed = fdm
        .sign_work_in(
            build_work_in_out(&config, 1, "75061189702", &booking),
            false,
        )
        .await
        .expect("work-in over mTLS");
    assert_eq!(signed.fdm_ref.fdm_id, "MCK00000001");
}

#[tokio::test]
async fn without_the_client_certificate_the_fdm_refuses_the_handshake() {
    let url = https_fdm();
    let fdm = FdmClient::from_config(&config(&url, mock_tls("ca.crt"), false));
    let err = fdm.status().await.unwrap_err();
    assert!(
        err.to_string().starts_with("FDM_CERTIFICATE_REFUSED:"),
        "{err}"
    );
}

#[tokio::test]
async fn an_fdm_signed_by_another_ca_is_refused() {
    let url = https_fdm();
    let module2_ca = format!(
        "{}/tests/fixtures/module2_tls/ca.crt",
        env!("CARGO_MANIFEST_DIR")
    );
    let fdm = FdmClient::from_config(&config(&url, module2_ca, true));
    let err = fdm.status().await.unwrap_err();
    assert!(
        format!("{err:?}").to_lowercase().contains("certificate"),
        "{err:?}"
    );
}
