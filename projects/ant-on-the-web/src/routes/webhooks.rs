use ant_library::routes::Routes;
use axum::{
    body::Bytes,
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json,
};
use serde_json::json;
use stripe::{EventObject, EventType, IssuingAuthorization, Webhook};
use tracing::debug;

use crate::{err::AntOnTheWebError, state::ApiRoutes};

const STRIPE_API_VERSION: &str = "2025-03-31.basil";

pub fn routes() -> ApiRoutes {
    Routes::new().post("/stripe", post(stripe_webhook))
}

async fn stripe_webhook(headers: HeaderMap, body: Bytes) -> Result<Response, AntOnTheWebError> {
    let secret = ant_library::secret::load_secret("stripe-webhook")?;

    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(AntOnTheWebError::WebhookSignatureError)?;

    let payload =
        std::str::from_utf8(&body).map_err(|_| AntOnTheWebError::WebhookSignatureError)?;

    let event = Webhook::construct_event(payload, sig, &secret)
        .map_err(|_| AntOnTheWebError::WebhookSignatureError)?;

    match event.type_ {
        EventType::IssuingAuthorizationRequest => {
            let EventObject::IssuingAuthorization(auth) = event.data.object else {
                return Ok(StatusCode::BAD_REQUEST.into_response());
            };

            let approved = decide(&auth);
            let mut resp = Json(json!({ "approved": approved })).into_response();
            resp.headers_mut().insert(
                HeaderName::from_static("stripe-version"),
                HeaderValue::from_static(STRIPE_API_VERSION),
            );
            Ok(resp)
        }

        EventType::PaymentIntentSucceeded => {
            if let EventObject::PaymentIntent(pi) = event.data.object {
                debug!(id = %pi.id, "payment_intent.succeeded");
            }
            Ok(StatusCode::OK.into_response())
        }

        other => {
            debug!(?other, "unhandled event type");
            Ok(StatusCode::OK.into_response())
        }
    }
}

fn decide(_auth: &IssuingAuthorization) -> bool {
    true
}
