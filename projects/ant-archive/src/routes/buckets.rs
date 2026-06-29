use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;

use crate::{auth::BearerClaims, err::AntArchiveError, state::AntArchiveState};

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Visibility {
    Public,
    Internal,
    Private,
}

impl Visibility {
    fn from_read_policy(policy: &str) -> Result<Self, AntArchiveError> {
        match policy {
            "public" => Ok(Self::Public),
            "internal" => Ok(Self::Internal),
            "private" => Ok(Self::Private),
            _ => Err(AntArchiveError::InternalServerError(
                "ANT-ERR-131",
                Some(anyhow::anyhow!("unknown read_policy: {policy}")),
            )),
        }
    }
}

#[derive(Serialize)]
struct Bucket {
    bucket_id: String,
    visibility: Visibility,
}

#[derive(Serialize)]
struct BucketList {
    buckets: Vec<Bucket>,
}

async fn list_buckets(
    State(state): State<AntArchiveState>,
    auth: BearerClaims,
) -> Result<impl IntoResponse, AntArchiveError> {
    let buckets = state
        .db
        .list_buckets_for_client(&auth.client_id)
        .await?
        .into_iter()
        .map(|b| {
            Ok(Bucket {
                visibility: Visibility::from_read_policy(&b.read_policy)?,
                bucket_id: b.bucket_id,
            })
        })
        .collect::<Result<Vec<_>, AntArchiveError>>()?;
    Ok(Json(BucketList { buckets }))
}

pub fn make_routes(state: AntArchiveState) -> Router {
    use ant_library::routes::Routes;

    Routes::new()
        .get("/", get(list_buckets))
        .build()
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(ant_library::middleware::http_log_layer())
                .layer(CatchPanicLayer::custom(
                    ant_library::middleware::catch_panic,
                ))
                .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(
                    ant_library::middleware::print_request_response,
                ))),
        )
}
