use std::sync::Arc;

use crate::{handle_throttling_error, state::ApiRoutes};
use ant_library::routes::Routes;
use axum::{extract::State, routing::post};
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::{
    err::AntOnTheWebError,
    routes::lib::{
        auth::{authenticate, AuthClaims},
        response::AntOnTheWebResponse,
    },
    state::{ApiState, InnerApiState},
    throttle::UserIdExtractor,
};

// #[debug_handler]
async fn print_message(
    State(InnerApiState { dao, sd, .. }): ApiState,
    auth: AuthClaims,
    body: String,
) -> Result<AntOnTheWebResponse, AntOnTheWebError> {
    let _ = authenticate(&auth, &dao).await?;

    let client = reqwest::Client::new();

    let endpoint =
        sd.resolve("ant-printing-press")
            .await
            .ok_or(AntOnTheWebError::InternalServerError(Some(
                anyhow::Error::msg("no endpoint found for ant-printing-press"),
            )))?;

    client
        .post(format!(
            "http://{}:{}/print/msg",
            endpoint.address, endpoint.port
        ))
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    Ok(AntOnTheWebResponse::PrintMessageResponse)
}

pub fn routes() -> ApiRoutes {
    let throttling = Arc::new(
        GovernorConfigBuilder::default()
            .period(std::time::Duration::from_hours(24))
            .burst_size(1)
            .use_headers()
            .key_extractor(UserIdExtractor) // Limit based on User ID, authenticated routes only
            .error_handler(|err| handle_throttling_error(&err))
            .finish()
            .unwrap(),
    );

    Routes::new().post(
        "/msg",
        post(print_message)
            .layer(ServiceBuilder::new().layer(GovernorLayer { config: throttling })),
    )
}
