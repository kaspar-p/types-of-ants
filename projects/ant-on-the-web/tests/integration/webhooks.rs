use crate::fixture::{FixtureOptions, TestFixture};
use http::StatusCode;
use serde_json::json;

fn make_stripe_signature(payload: &str, secret: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    make_stripe_signature_at(payload, secret, timestamp)
}

fn make_stripe_signature_at(payload: &str, secret: &str, timestamp: u64) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let signed_payload = format!("{}.{}", timestamp, payload);
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize();
    format!("t={},v1={}", timestamp, hex::encode(result.into_bytes()))
}

const TEST_SECRET: &str = "test-stripe-webhook-secret";

#[tokio::test]
async fn stripe_webhook_returns_400_missing_signature() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .body("{}".to_string())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stripe_webhook_returns_400_invalid_signature() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", "bad-value")
        .body("{}".to_string())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// Well-formed `t=...,v1=<64 hex>` signature signed with the wrong secret. Unlike
// `bad-value` above (which fails signature parsing), this reaches and exercises
// the HMAC verification itself.
#[tokio::test]
async fn stripe_webhook_returns_400_wrong_secret() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let payload = "{}";
    let sig = make_stripe_signature(payload, "the-wrong-secret");

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(payload.to_string())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// Signature computed over one body, but a different body is sent. Verifies the
// signature actually protects body integrity, not just header presence.
#[tokio::test]
async fn stripe_webhook_returns_400_tampered_body() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let signed_body = json!({ "id": "evt_signed", "object": "event" }).to_string();
    let sig = make_stripe_signature(&signed_body, TEST_SECRET);

    let tampered_body = json!({ "id": "evt_tampered", "object": "event" }).to_string();

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(tampered_body)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// Correctly signed with the right secret, but the signature timestamp is older
// than Stripe's 5-minute tolerance, so verification must reject it (replay).
#[tokio::test]
async fn stripe_webhook_returns_400_expired_timestamp() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let payload = "{}";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let stale_timestamp = now - 600; // 10 minutes ago, outside the 300s window
    let sig = make_stripe_signature_at(payload, TEST_SECRET, stale_timestamp);

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(payload.to_string())
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stripe_webhook_returns_200_issuing_authorization_request() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let payload = json!({
        "id": "evt_test_auth",
        "object": "event",
        "api_version": "2025-03-31.basil",
        "type": "issuing_authorization.request",
        "created": 1700000000,
        "livemode": false,
        "pending_webhooks": 0,
        "data": {
            "object": {
                "id": "iauth_test_123",
                "object": "issuing.authorization",
                "amount": 100,
                "amount_details": null,
                "approved": false,
                "authorization_method": "online",
                "balance_transactions": [],
                "card": {
                    "id": "ic_test_123",
                    "object": "issuing.card",
                    "brand": "Visa",
                    "cancellation_reason": null,
                    "cardholder": {
                        "id": "ich_test_123",
                        "object": "issuing.cardholder",
                        "billing": {
                            "address": {
                                "city": "Anytown",
                                "country": "US",
                                "line1": "123 Main St",
                                "line2": null,
                                "postal_code": "12345",
                                "state": "CA"
                            }
                        },
                        "created": 1700000000,
                        "email": null,
                        "livemode": false,
                        "metadata": {},
                        "name": "Test Cardholder",
                        "phone_number": null,
                        "redaction": null,
                        "requirements": {
                            "disabled_reason": null,
                            "past_due": []
                        },
                        "spending_controls": {
                            "allowed_categories": null,
                            "blocked_categories": null,
                            "spending_limits": [],
                            "spending_limits_currency": null
                        },
                        "status": "active",
                        "type": "individual"
                    },
                    "created": 1700000000,
                    "currency": "usd",
                    "exp_month": 12,
                    "exp_year": 2030,
                    "financial_account": null,
                    "last4": "4242",
                    "livemode": false,
                    "metadata": {},
                    "replaced_by": null,
                    "replacement_for": null,
                    "replacement_reason": null,
                    "shipping": null,
                    "spending_controls": {
                        "allowed_categories": null,
                        "blocked_categories": null,
                        "spending_limits": [],
                        "spending_limits_currency": null
                    },
                    "status": "active",
                    "type": "virtual",
                    "wallets": null
                },
                "cardholder": null,
                "created": 1700000000,
                "currency": "usd",
                "livemode": false,
                "merchant_amount": 100,
                "merchant_currency": "usd",
                "merchant_data": {
                    "category": "computer_programming",
                    "category_code": "7372",
                    "city": "San Francisco",
                    "country": "US",
                    "name": "Test Merchant",
                    "network_id": "net_test_123",
                    "postal_code": "94107",
                    "state": "CA",
                    "terminal_id": null,
                    "url": null
                },
                "metadata": {},
                "network_data": null,
                "pending_request": null,
                "request_history": [],
                "status": "pending",
                "token": null,
                "transactions": [],
                "verification_data": {
                    "address_line1_check": "not_provided",
                    "address_postal_code_check": "not_provided",
                    "authentication_exemption": null,
                    "cvc_check": "not_provided",
                    "expiry_check": "match",
                    "three_d_secure": null
                },
                "wallet": null
            }
        }
    });

    let payload_str = serde_json::to_string(&payload).unwrap();
    let sig = make_stripe_signature(&payload_str, TEST_SECRET);

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(payload_str)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers().contains_key("stripe-version"),
        "stripe-version header should be present"
    );
    let body: serde_json::Value = res.json().await;
    assert_eq!(body, json!({ "approved": true }));
}

#[tokio::test]
async fn stripe_webhook_returns_200_payment_intent_succeeded() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let payload = json!({
        "id": "evt_test_123",
        "object": "event",
        "api_version": "2025-03-31.basil",
        "type": "payment_intent.succeeded",
        "created": 1700000000,
        "livemode": false,
        "pending_webhooks": 0,
        "data": {
            "object": {
                "id": "pi_test_123",
                "object": "payment_intent",
                "amount": 1000,
                "amount_capturable": 0,
                "amount_received": 1000,
                "capture_method": "automatic",
                "confirmation_method": "automatic",
                "created": 1700000000,
                "currency": "usd",
                "livemode": false,
                "metadata": {},
                "payment_method_types": ["card"],
                "status": "succeeded"
            }
        }
    });

    let payload_str = serde_json::to_string(&payload).unwrap();
    let sig = make_stripe_signature(&payload_str, TEST_SECRET);

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(payload_str)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn stripe_webhook_returns_200_unknown_event() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    // Use an unrecognized event type so EventType::Unknown is set.
    // The data.object must still be a known Stripe object type (we use "account"
    // which has no required fields beyond id) so EventObject deserialization succeeds.
    let payload = json!({
        "id": "evt_test_unknown",
        "object": "event",
        "api_version": "2025-03-31.basil",
        "type": "totally_unknown.event_type",
        "created": 1700000000,
        "livemode": false,
        "pending_webhooks": 0,
        "data": {
            "object": {
                "id": "acct_test_123",
                "object": "account"
            }
        }
    });

    let payload_str = serde_json::to_string(&payload).unwrap();
    let sig = make_stripe_signature(&payload_str, TEST_SECRET);

    let res = fixture
        .client
        .post("/api/webhooks/stripe")
        .header("stripe-signature", sig.as_str())
        .header("content-type", "application/json")
        .body(payload_str)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);
}
