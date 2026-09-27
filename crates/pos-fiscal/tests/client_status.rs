//! FdmClient status-query contract tests (Q100 / Q110).
//!
//! Uses wiremock to stand in for the FDM and verifies:
//!
//! 1. `status_with_language` interpolates the requested language and
//!    falls back from the SDL spelling (`Status`) to the UC-fixture
//!    spelling (`status`) when the FDM rejects the capitalised field.
//! 2. `status_multi_lang` issues the Q110-shaped aliased query and
//!    deserializes the multi-key envelope into `MultiLangStatus`.

use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::types::Language;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn status_payload() -> serde_json::Value {
    serde_json::json!({
        "device": {
            "fdmId": "FOD01000001",
            "fdmSwVersion": "1.2.0",
            "fdmDateTime": "2026-07-03T10:00:39.514Z",
            "bufferCapacityUsed": 10.02
        },
        "initialized": true,
        "informations": [],
        "warnings": [],
        "errors": []
    })
}

#[tokio::test]
async fn status_with_language_passes_language_argument() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("Status(language: NL)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "fdmStatus": status_payload() }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = FdmClient::new(&format!("{}/graphql", server.uri()));
    let status = client
        .status_with_language(Language::Nl)
        .await
        .expect("status with NL");
    assert_eq!(status.device.fdm_id, "FOD01000001");
}

#[tokio::test]
async fn status_falls_back_to_lowercase_spelling() {
    let server = MockServer::start().await;
    // The capitalised field is rejected like a strict UC-fixture FDM
    // would: GraphQL error, no data.
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("Status(language: FR)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": null,
            "errors": [{ "message": "Cannot query field \"Status\" on type \"Query\"" }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("status(language: FR)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "fdmStatus": status_payload() }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = FdmClient::new(&format!("{}/graphql", server.uri()));
    let status = client.status().await.expect("fallback to lowercase");
    assert!(status.initialized);
}

#[tokio::test]
async fn status_multi_lang_parses_q110_envelope() {
    let server = MockServer::start().await;
    let msgs = |text: &str| {
        serde_json::json!({
            "informations": [],
            "warnings": [{
                "message": text,
                "extensions": {
                    "category": "SPF_FOD",
                    "code": "BUFFER_NEAR_FULL",
                    "showPos": "MANDATORY"
                }
            }],
            "errors": []
        })
    };
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("messagesFR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "fdmStatus": status_payload(),
                "messagesFR": msgs("L'utilisation du buffer du FDM dépasse 70%"),
                "messagesNL": msgs("Het buffergebruik van de FDM overschrijdt 70%"),
                "messagesDE": msgs("Die Pufferauslastung des FDM übersteigt 70%")
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = FdmClient::new(&format!("{}/graphql", server.uri()));
    let multi = client.status_multi_lang().await.expect("Q110 query");

    assert_eq!(multi.status.device.fdm_id, "FOD01000001");
    let fr = multi.messages_fr.warnings.expect("FR warnings");
    let nl = multi.messages_nl.warnings.expect("NL warnings");
    let de = multi.messages_de.warnings.expect("DE warnings");
    assert!(matches!(
        fr[0].extensions.code,
        pos_fiscal::types::FdmWarningCode::BufferNearFull
    ));
    // Each alias carries its own localization.
    assert_ne!(fr[0].message, nl[0].message);
    assert_ne!(nl[0].message, de[0].message);
}

/// The two codes of the POS description v2.2 our first list lacked, and
/// a code outside the spec (another FDM version, a vendor code): the
/// Status still parses, so the FDM is not shown as down.
#[tokio::test]
async fn status_with_new_and_unknown_codes_still_parses() {
    use pos_fiscal::types::{FdmWarningCode, MessageCategory};
    let server = MockServer::start().await;
    let msg = |category: &str, code: &str| {
        serde_json::json!({
            "message": code,
            "extensions": { "category": category, "code": code, "showPos": "MANDATORY" }
        })
    };
    let mut body = status_payload();
    body["warnings"] = serde_json::json!([
        msg("SPF_FOD", "UPDATE_POS_VATNO"),
        msg("SPF_FOD", "UPDATE_POS_ESTNO"),
        msg("VENDOR", "MODULE2_SOMETHING_NEW"),
    ]);
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "data": { "fdmStatus": body } })),
        )
        .mount(&server)
        .await;

    let status = FdmClient::new(&format!("{}/graphql", server.uri()))
        .status()
        .await
        .expect("status parses");
    let warnings = status.warnings.expect("warnings");
    let codes: Vec<_> = warnings.iter().map(|w| w.extensions.code).collect();
    assert_eq!(
        codes,
        [
            FdmWarningCode::UpdatePosVatno,
            FdmWarningCode::UpdatePosEstno,
            FdmWarningCode::Unrecognized
        ]
    );
    assert_eq!(warnings[2].extensions.category, MessageCategory::Other);
}
