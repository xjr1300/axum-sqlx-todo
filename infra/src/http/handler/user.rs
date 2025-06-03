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
use use_case::user::{LoginInput, LoginOutput, UserUseCase};

use super::{ApiError, ApiResult, serialize_option_offset_datetime, serialize_secret_string};
use crate::{
    AppState, postgres::repositories::PgUserRepository, redis::token::RedisTokenRepository,
};

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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
    let use_case = user_use_case(&app_state);
    let user = use_case
        .sign_up(input, hashed_password)
        .await
        .map_err(ApiError::from)?;
    let response_body = UserResponseBody::from(user);
    Ok(Json(response_body))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
    email: String,
    password: SecretString,
}

impl TryFrom<LoginRequestBody> for LoginInput {
    type Error = DomainError;

    fn try_from(input: LoginRequestBody) -> DomainResult<Self> {
        Ok(LoginInput {
            email: Email::new(input.email)?,
            raw_password: RawPassword::new(input.password)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseBody {
    #[serde(serialize_with = "serialize_secret_string")]
    access_token: SecretString,
    #[serde(serialize_with = "rfc3339::serialize")]
    access_expiration: OffsetDateTime,
    #[serde(serialize_with = "serialize_secret_string")]
    refresh_token: SecretString,
    #[serde(serialize_with = "rfc3339::serialize")]
    refresh_expiration: OffsetDateTime,
}

impl From<LoginOutput> for LoginResponseBody {
    fn from(output: LoginOutput) -> Self {
        Self {
            access_token: output.access_token.0,
            access_expiration: output.access_expiration,
            refresh_token: output.refresh_token.0,
            refresh_expiration: output.refresh_expiration,
        }
    }
}

/// ログインハンドラ
pub async fn login(
    State(app_state): State<AppState>,
    Json(request_body): Json<LoginRequestBody>,
) -> ApiResult<Json<LoginResponseBody>> {
    let settings = &app_state.app_settings;
    let input = LoginInput::try_from(request_body).map_err(ApiError::from)?;
    let use_case = user_use_case(&app_state);
    let output = use_case
        .login(input, &settings.password, &settings.login, &settings.token)
        .await
        .map_err(ApiError::from)?;
    let response_body = LoginResponseBody::from(output);
    Ok(Json(response_body))
}

fn user_use_case(app_state: &AppState) -> UserUseCase<PgUserRepository, RedisTokenRepository> {
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    UserUseCase::new(user_repo, token_repo)
}
