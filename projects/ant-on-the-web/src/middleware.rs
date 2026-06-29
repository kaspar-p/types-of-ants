use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use http::HeaderValue;
use tracing::error;

use crate::state::InnerApiState;

pub async fn x_ant_middleware(
    State(state): State<InnerApiState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut res = next.run(req).await;

    match state.dao.ants.get_random_released_name().await {
        Ok(Some(name)) => {
            let sanitized: String = name
                .chars()
                .filter(|c| ('\x20'..='\x7e').contains(c))
                .collect();

            if !sanitized.is_empty() {
                if let Ok(value) = HeaderValue::from_str(&sanitized) {
                    res.headers_mut().insert("x-ant", value);
                }
            }
        }
        Ok(None) => {}
        Err(e) => error!("ANT-ERR-049: x-ant: failed to fetch random ant: {e}"),
    }

    res
}
