use axum::{
    body::Bytes,
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use stripe::{EventObject, EventType, IssuingAuthorization, Webhook};
use tracing::{debug, warn};

use crate::state::ApiRouter;

const STRIPE_API_VERSION: &str = "2025-03-31.basil";

pub fn router() -> ApiRouter {
    Router::new().route("/stripe", post(stripe_webhook))
}

async fn stripe_webhook(headers: HeaderMap, body: Bytes) -> Response {
    let secret = match ant_library::secret::load_secret("stripe-webhook") {
        Ok(s) => s,
        Err(e) => {
            warn!(error = ?e, "failed to load stripe-webhook secret");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Some(sig) = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Ok(payload) = std::str::from_utf8(&body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let event = match Webhook::construct_event(payload, sig, &secret) {
        Ok(event) => event,
        Err(e) => {
            warn!(error = ?e, "webhook signature verification failed");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    match event.type_ {
        EventType::IssuingAuthorizationRequest => {
            let EventObject::IssuingAuthorization(auth) = event.data.object else {
                return StatusCode::BAD_REQUEST.into_response();
            };

            let approved = decide(&auth);
            let mut resp = Json(json!({ "approved": approved })).into_response();
            resp.headers_mut().insert(
                HeaderName::from_static("stripe-version"),
                HeaderValue::from_static(STRIPE_API_VERSION),
            );
            resp
        }

        EventType::PaymentIntentSucceeded => {
            if let EventObject::PaymentIntent(pi) = event.data.object {
                debug!(id = %pi.id, "payment_intent.succeeded");
            }
            StatusCode::OK.into_response()
        }

        other => {
            debug!(?other, "unhandled event type");
            StatusCode::OK.into_response()
        }
    }
}

fn decide(_auth: &IssuingAuthorization) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sig(payload: &str, secret: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let signed = format!("{}.{}", ts, payload);
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let result = mac.finalize();
        format!("t={},v1={}", ts, hex::encode(result.into_bytes()))
    }

    #[test]
    fn test_unknown_event_parse() {
        let payload = r#"{"id":"evt_test_unknown","object":"event","api_version":"2025-03-31.basil","type":"totally_unknown.event_type","created":1700000000,"livemode":false,"pending_webhooks":0,"data":{"object":{"id":"acct_test_123","object":"account"}}}"#;
        let secret = "test-secret";
        let sig = make_sig(payload, secret);
        let result = Webhook::construct_event(payload, &sig, secret);
        eprintln!("unknown event result: {:?}", result);
        assert!(result.is_ok(), "expected ok but got: {:?}", result);
    }

    #[test]
    fn test_issuing_auth_event_parse() {
        let payload = r#"{"id":"evt_test_auth","object":"event","api_version":"2025-03-31.basil","type":"issuing_authorization.request","created":1700000000,"livemode":false,"pending_webhooks":0,"data":{"object":{"id":"iauth_test_123","object":"issuing.authorization","amount":100,"amount_details":null,"approved":false,"authorization_method":"online","balance_transactions":[],"card":{"id":"ic_test_123","object":"issuing.card","brand":"Visa","cancellation_reason":null,"cardholder":{"id":"ich_test_123","object":"issuing.cardholder","billing":{"address":{"city":"Anytown","country":"US","line1":"123 Main St","line2":null,"postal_code":"12345","state":"CA"}},"created":1700000000,"email":null,"livemode":false,"metadata":{},"name":"Test Cardholder","phone_number":null,"redaction":null,"requirements":{"disabled_reason":null,"past_due":[]},"spending_controls":{"allowed_categories":null,"blocked_categories":null,"spending_limits":[],"spending_limits_currency":null},"status":"active","type":"individual"},"created":1700000000,"currency":"usd","exp_month":12,"exp_year":2030,"financial_account":null,"last4":"4242","livemode":false,"metadata":{},"replaced_by":null,"replacement_for":null,"replacement_reason":null,"shipping":null,"spending_controls":{"allowed_categories":null,"blocked_categories":null,"spending_limits":[],"spending_limits_currency":null},"status":"active","type":"virtual","wallets":null},"cardholder":null,"created":1700000000,"currency":"usd","livemode":false,"merchant_amount":100,"merchant_currency":"usd","merchant_data":{"category":"computer_programming","category_code":"7372","city":"San Francisco","country":"US","name":"Test Merchant","network_id":"net_test_123","postal_code":"94107","state":"CA","terminal_id":null,"url":null},"metadata":{},"network_data":null,"pending_request":null,"request_history":[],"status":"pending","token":null,"transactions":[],"verification_data":{"address_line1_check":"not_provided","address_postal_code_check":"not_provided","authentication_exemption":null,"cvc_check":"not_provided","expiry_check":"match","three_d_secure":null},"wallet":null}}}"#;
        let secret = "test-secret";
        let sig = make_sig(payload, secret);
        let result = Webhook::construct_event(payload, &sig, secret);
        eprintln!("issuing auth event result: {:?}", result);
        assert!(result.is_ok(), "expected ok but got: {:?}", result);
    }

    #[test]
    fn test_payment_intent_event_parse() {
        let payload = r#"{"id":"evt_test_123","object":"event","api_version":"2025-03-31.basil","type":"payment_intent.succeeded","created":1700000000,"livemode":false,"pending_webhooks":0,"data":{"object":{"id":"pi_test_123","object":"payment_intent","amount":1000,"amount_capturable":0,"amount_received":1000,"capture_method":"automatic","confirmation_method":"automatic","created":1700000000,"currency":"usd","livemode":false,"metadata":{},"payment_method_types":["card"],"status":"succeeded"}}}"#;
        let secret = "test-secret";
        let sig = make_sig(payload, secret);
        let result = Webhook::construct_event(payload, &sig, secret);
        eprintln!("result: {:?}", result);
        assert!(result.is_ok(), "expected ok but got: {:?}", result);
    }
}
