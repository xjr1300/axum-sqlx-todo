use axum::{Json, extract::State};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, serde::rfc3339};

use domain::{
    DomainError, DomainResult,
    models::{Email, FamilyName, GivenName, RawPassword, User},
    password::create_hashed_password,
    repositories::UserInput,
};
use use_case::user::UserUseCase;

use super::{ApiError, ApiResult, serialize_option_offset_datetime};
use crate::{AppState, postgres::repositories::PgUserRepository};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpRequestBody {
    family_name: String,
    given_name: String,
    email: String,
    password: SecretString,
}

impl TryFrom<SignUpRequestBody> for UserInput {
    type Error = DomainError;

    fn try_from(input: SignUpRequestBody) -> DomainResult<Self> {
        Ok(UserInput {
            family_name: FamilyName::new(input.family_name)?,
            given_name: GivenName::new(input.given_name)?,
            email: Email::new(input.email)?,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponseBody {
    id: String,
    family_name: String,
    given_name: String,
    email: String,
    active: bool,
    #[serde(
        serialize_with = "serialize_option_offset_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    last_login_at: Option<OffsetDateTime>,
    #[serde(serialize_with = "rfc3339::serialize")]
    created_at: OffsetDateTime,
    #[serde(serialize_with = "rfc3339::serialize")]
    updated_at: OffsetDateTime,
}

impl From<User> for UserResponseBody {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            family_name: user.family_name.0,
            given_name: user.given_name.0,
            email: user.email.0,
            active: user.active,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// サインアップハンドラ
pub async fn sign_up(
    State(app_state): State<AppState>,
    Json(request_body): Json<SignUpRequestBody>,
) -> ApiResult<Json<UserResponseBody>> {
    // パスワードの検証とハッシュ化
    let raw_password = RawPassword::new(request_body.password.clone()).map_err(ApiError::from)?;
    let hashed_password = create_hashed_password(&app_state.app_settings.password, &raw_password)
        .map_err(ApiError::from)?;

    // リクエストボディをUserInputに変換
    let input = UserInput::try_from(request_body).map_err(ApiError::from)?;

    // ユーザーを登録
    let repository = PgUserRepository::new(app_state.pool.clone());
    let use_case = UserUseCase::new(repository);
    let user = use_case
        .sign_up(input, hashed_password)
        .await
        .map_err(ApiError::from)?;
    let response_body = UserResponseBody::from(user);
    Ok(Json(response_body))
}
