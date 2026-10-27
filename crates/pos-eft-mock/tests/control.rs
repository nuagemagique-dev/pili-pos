//! The control API and the page, over real HTTP.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_eft_mock::{Behaviour, Config, Mock, Next};
use serde_json::{Value, json};

fn worldline_body(reference: &str, amount: f64) -> Value {
    json!({"SaleToPOIServiceRequest": {
        "Header": {
            "MessageFunction": "SaleFinancialServiceRequest",
            "ProtocolVersion": "5.1-WL2.1.1",
            "ExchangeIdentification": reference,
            "CreationDateTime": "2026-10-12T11:00:00.000+02:00",
            "InitiatingParty": {"Identification": "WL000D5T0000004", "Type": "Merchant"},
            "SalesSystemInfo": {"IntegratorId": "x", "SaleSoftware": [
                {"Type": "ECR", "Status": {"VersionNumber": "1.0.0"}}
            ]},
        },
        "ServiceRequest": {
            "Environment": {"POI": {"Identification": {"Identification": "26197214"}}},
            "ServiceContent": "FinancialPaymentRequest",
            "PaymentRequest": {"PaymentTransaction": {
                "TransactionType": "CardPayment",
                "TransactionIdentification": {
                    "TransactionDateTime": "2026-10-12T11:00:00.000+02:00",
                    "TransactionReference": reference,
                },
                "TransactionDetails": {"Currency": "EUR", "TotalAmount": amount},
            }},
        },
    }})
}

const REF: &str = "6f1f0a52-0000-4000-8000-000000000001";

async fn pay(client: &reqwest::Client, mock: &Mock, body: &Value) -> reqwest::Response {
    client
        .post(format!(
            "{}/api/v2/merchants/WL000D5T0000004/terminals/26197214/payments/sync",
            mock.url()
        ))
        .json(body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn next_sets_the_behaviour_of_one_payment_and_requests_are_listed() {
    let mock = Mock::start();
    let client = reqwest::Client::new();
    let set: Value = client
        .post(format!("{}/mock/next", mock.url()))
        .json(&json!({"result": "decline", "brand": "Visa", "delay_ms": 0}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(set["result"], "decline");
    assert_eq!(set["brand"], "Visa");

    let answer: Value = pay(&client, &mock, &worldline_body(REF, 12.5))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        answer["data"]["SaleToPOIServiceResponse"]["ServiceResponse"]["Response"]["ResponseReason"],
        "Refusal"
    );

    // The next one is back to the default: approved.
    mock.set_default(Next {
        delay_ms: 0,
        ..Next::default()
    });
    let second = "6f1f0a52-0000-4000-8000-000000000002";
    let answer: Value = pay(&client, &mock, &worldline_body(second, 3.0))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        answer["data"]["SaleToPOIServiceResponse"]["ServiceResponse"]["Response"]["Response"],
        "Success"
    );

    let requests: Vec<Value> = client
        .get(format!("{}/mock/requests", mock.url()))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["protocol"], "worldline");
    assert_eq!(
        requests[1]["body"]["SaleToPOIServiceRequest"]["Header"]["ExchangeIdentification"],
        second
    );
    assert_eq!(requests[0]["status"], 200);

    let payments: Vec<Value> = client
        .get(format!("{}/mock/payments", mock.url()))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(payments[0]["outcome"], "declined");
    assert_eq!(payments[1]["outcome"], "approved");
    assert_eq!(payments[0]["brand"], "Visa");

    let page = client
        .get(mock.url())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("Payment terminal mock")
            && page.contains("declined")
            && page.contains("EUR 12.50")
    );

    let reset = client
        .post(format!("{}/mock/reset", mock.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(reset.status(), 204);
    assert!(mock.payments().is_empty());
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn the_wire_checks_of_the_real_apis_are_enforced() {
    let mock = Mock::start();
    let client = reqwest::Client::new();
    // Worldline v2: ExchangeIdentification must be a UUID.
    let r = pay(&client, &mock, &worldline_body("642", 1.0)).await;
    assert_eq!(r.status(), 400);
    let body: Value = r.json().await.unwrap();
    assert!(body["message"].as_str().unwrap().contains("UUID"));
    // A key is demanded once configured.
    mock.set_api_key(Some("k"));
    assert_eq!(
        pay(&client, &mock, &worldline_body(REF, 1.0))
            .await
            .status(),
        401
    );
    // CCV: Basic with the key as user and an empty password.
    let ccv = client
        .get(format!("{}/api/v1/transaction?reference=TL1", mock.url()))
        .basic_auth("k", Some(""))
        .send()
        .await
        .unwrap();
    assert_eq!(
        ccv.status(),
        404,
        "authenticated, but the reference is unknown"
    );
    let ccv = client
        .get(format!("{}/api/v1/transaction?reference=TL1", mock.url()))
        .basic_auth("other", Some(""))
        .send()
        .await
        .unwrap();
    assert_eq!(ccv.status(), 401);
    // Unknown paths answer 404, and invalid control bodies 400.
    assert_eq!(
        client
            .get(format!("{}/nope", mock.url()))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let r = client
        .post(format!("{}/mock/next", mock.url()))
        .body(r#"{"result":"explode"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}

#[test]
fn the_defaults_are_the_documented_ones() {
    // Nothing is set in the test environment: the documented ports.
    let c = Config::from_env().unwrap();
    assert_eq!(c.http.port(), 19400);
    assert_eq!(c.opi.port(), 19410);
    assert_eq!(c.opi_device_port, 4102);
    let next = Next::default();
    assert_eq!(next.result, Behaviour::Approve);
    assert_eq!(next.brand, "Bancontact");
    assert_eq!(next.delay_ms, 1500);
}
