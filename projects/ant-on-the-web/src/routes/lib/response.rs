use axum::{response::IntoResponse, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    ants::{
        AllAntsResponse, CreateReleaseResponse, DeclineAntResponse, DeclinedAntsResponse,
        FavoriteAntResponse, GetReleaseResponse, LatestAntsResponse, LatestReleaseResponse,
        ReleasedAntsResponse, SuggestionResponse, TotalResponse, UnreleasedAntsResponse,
    },
    users::{
        AddEmailResponse, AddPhoneNumberResponse, GetUserResponse, LoginResponse,
        PasswordResetCodeResponse, PasswordResetSecretResponse, SignupResponse,
        VerificationAttemptResponse,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "__type")]
pub enum AntOnTheWebResponse {
    LoginResponse(LoginResponse),
    LogoutResponse,
    SignupResponse(SignupResponse),
    SubscribeNewsletterResponse,
    GetUserResponse(GetUserResponse),
    ChangeUsernameResponse,
    VerificationAttemptResponse(VerificationAttemptResponse),
    AddPhoneNumberResponse(AddPhoneNumberResponse),
    AddEmailResponse(AddEmailResponse),
    PasswordResetCodeResponse(PasswordResetCodeResponse),
    PasswordResetSecretResponse(PasswordResetSecretResponse),
    PasswordResetResponse,

    AllAntsResponse(AllAntsResponse),
    UnreleasedAntsResponse(UnreleasedAntsResponse),
    DeclinedAntsResponse(DeclinedAntsResponse),
    ReleasedAntsResponse(ReleasedAntsResponse),
    LatestReleaseResponse(LatestReleaseResponse),
    GetReleaseResponse(GetReleaseResponse),
    CreateReleaseResponse(CreateReleaseResponse),
    TotalResponse(TotalResponse),
    LatestAntsResponse(LatestAntsResponse),
    SuggestionResponse(SuggestionResponse),
    DeclineAntResponse(DeclineAntResponse),
    FavoriteAntResponse(FavoriteAntResponse),
    UnfavoriteAntResponse,

    WebActionResponse,
}

impl IntoResponse for AntOnTheWebResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
