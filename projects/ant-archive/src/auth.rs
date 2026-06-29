use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use http::request::Parts;

use ant_archive_db::ClientCapabilities;

use crate::{err::AntArchiveError, state::AntArchiveState};

pub struct BearerClaims {
    pub client_id: String,
    pub capabilities: ClientCapabilities,
}

/// Required auth — fails with 401 if no valid bearer token is present.
impl<S> FromRequestParts<S> for BearerClaims
where
    AntArchiveState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AntArchiveError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(auth) =
            <TypedHeader<Authorization<Bearer>> as FromRequestParts<S>>::from_request_parts(
                parts, state,
            )
            .await
            .map_err(|e| AntArchiveError::Unauthorized(Some(e.into())))?;

        let state = AntArchiveState::from_ref(state);
        let (client_id, capabilities) = state
            .db
            .authenticate_bearer(auth.token())
            .await
            .map_err(AntArchiveError::from)?
            .ok_or(AntArchiveError::Unauthorized(None))?;

        Ok(BearerClaims { client_id, capabilities })
    }
}

/// Optional auth — returns None when no bearer token is present or when the token is not
/// recognised. Silently treating an unrecognised token as no-auth is intentional: it prevents
/// private-bucket enumeration (callers cannot distinguish "bucket doesn't exist" from "you can't
/// access it").
impl<S> OptionalFromRequestParts<S> for BearerClaims
where
    AntArchiveState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AntArchiveError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let maybe_header =
            Option::<TypedHeader<Authorization<Bearer>>>::from_request_parts(parts, state)
                .await
                .map_err(|e| AntArchiveError::from(anyhow::anyhow!("{e}")))?;

        let Some(TypedHeader(auth)) = maybe_header else {
            return Ok(None);
        };

        let state = AntArchiveState::from_ref(state);
        let maybe_result = state
            .db
            .authenticate_bearer(auth.token())
            .await
            .map_err(AntArchiveError::from)?;

        Ok(maybe_result.map(|(client_id, capabilities)| BearerClaims { client_id, capabilities }))
    }
}
