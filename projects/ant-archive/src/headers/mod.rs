use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use http::request::Parts;

use crate::{auth::BearerClaims, err::AntArchiveError, state::AntArchiveState};

/// Value of `X-Ant-Capability-Can-Select-Storage-Node`, a node ID or hostname.
///
/// Resolves to `None` when the authenticated client lacks the capability or
/// the header is absent. Resolves to `None` (not an error) when there is no
/// bearer token at all — the caller's required `BearerClaims` extractor will
/// produce the 401.
pub struct SelectStorageNode(pub String);

impl<S> OptionalFromRequestParts<S> for SelectStorageNode
where
    AntArchiveState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AntArchiveError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let maybe_auth = Option::<BearerClaims>::from_request_parts(parts, state).await?;
        match maybe_auth {
            Some(auth) if auth.capabilities.can_select_storage_node => {}
            _ => return Ok(None),
        }

        Ok(parts
            .headers
            .get("x-ant-capability-can-select-storage-node")
            .and_then(|v| v.to_str().ok())
            .map(|s| SelectStorageNode(s.to_owned())))
    }
}
