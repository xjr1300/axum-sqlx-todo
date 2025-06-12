use std::borrow::Cow;

use axum::{
    Extension, Json,
    body::Body,
    extract::State,
    http::{HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use cookie::{Cookie, SameSite};
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, serde::rfc3339};

use domain::{
    DomainError, DomainResult,
    models::{Email, FamilyName, GivenName, RawPassword, User, UserId},
    repositories::{
        TokenRepository as _, TokenType, UpdateUserInput, UserInput, UserRepository,
        generate_auth_token_info, generate_auth_token_info_key,
    },
};
use use_case::{AuthorizedUser, user::UserUseCase};
use utils::serde::{deserialize_secret_string, serialize_secret_string};

use crate::{
    AppState,
    http::{
        ApiError, ApiResult, COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY, bad_request,
        internal_server_error, login_failed, unauthorized, user_locked,
    },
    jwt::generate_token_pair,
    password::{create_hashed_password, verify_password},
    postgres::repositories::PgUserRepository,
    redis::token::RedisTokenRepository,
    settings::{AppSettings, HttpProtocol},
};

type UserUseCaseImpl = UserUseCase<PgUserRepository, RedisTokenRepository>;

fn user_use_case(app_state: &AppState) -> UserUseCaseImpl {
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    UserUseCase::new(user_repo, token_repo)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpRequestBody {
    pub family_name: String,
    pub given_name: String,
    pub email: String,
    pub password: SecretString,
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

/// サインアップハンドラ
#[tracing::instrument(skip(app_state))]
pub async fn sign_up(
    State(app_state): State<AppState>,
    Json(request_body): Json<SignUpRequestBody>,
) -> ApiResult<Json<User>> {
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
    Ok(Json(user))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
    email: String,
    password: SecretString,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseBody {
    #[serde(serialize_with = "serialize_secret_string")]
    access_token: SecretString,
    #[serde(serialize_with = "rfc3339::serialize")]
    access_expired_at: OffsetDateTime,
    #[serde(serialize_with = "serialize_secret_string")]
    refresh_token: SecretString,
    #[serde(serialize_with = "rfc3339::serialize")]
    refresh_expired_at: OffsetDateTime,
}

/// ログインハンドラ
#[tracing::instrument(skip(app_state))]
pub async fn login(
    State(app_state): State<AppState>,
    Json(request_body): Json<LoginRequestBody>,
) -> ApiResult<Response<Body>> {
    let requested_at = OffsetDateTime::now_utc();
    let settings = &app_state.app_settings;
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());

    // Eメールアドレスからユーザーを取得
    let email =
        Email::new(request_body.email).map_err(|_| bad_request("Invalid email address".into()))?;
    let user = user_repo
        .by_email(&email)
        .await
        .map_err(internal_server_error)?
        .ok_or_else(login_failed)?;
    tracing::debug!("User found: {}", user.email);
    // ユーザーのアクティブフラグを確認
    if !user.active {
        return Err(user_locked());
    }
    tracing::debug!("User is active: {}", user.email);
    // ユーザーのハッシュ化されたパスワードを取得
    let hashed_password = user_repo
        .get_hashed_password(user.id)
        .await
        .map_err(internal_server_error)?;
    // ユーザーのパスワードを検証
    let raw_password = RawPassword::new(request_body.password).map_err(|_| login_failed())?;
    if verify_password(&raw_password, &settings.password.pepper, &hashed_password)
        .map_err(internal_server_error)?
    {
        tracing::debug!("Password is correct: {}", user.email);
        generate_tokens_response(settings, user_repo, token_repo, user.id, requested_at).await
    } else {
        handle_password_unmatched(settings, user_repo, user.id, requested_at).await
    }
}

#[tracing::instrument]
pub async fn me(Extension(user): Extension<AuthorizedUser>) -> ApiResult<Json<User>> {
    Ok(Json(user.0))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequestBody {
    pub family_name: Option<String>,
    pub given_name: Option<String>,
    pub email: Option<String>,
}

impl TryFrom<UpdateUserRequestBody> for UpdateUserInput {
    type Error = DomainError;

    fn try_from(input: UpdateUserRequestBody) -> DomainResult<Self> {
        Ok(UpdateUserInput {
            family_name: input.family_name.map(FamilyName::new).transpose()?,
            given_name: input.given_name.map(GivenName::new).transpose()?,
            email: input.email.map(Email::new).transpose()?,
        })
    }
}

/// ログアウトハンドラ
#[tracing::instrument(skip(app_state))]
pub async fn update(
    State(app_state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
    Json(request_body): Json<UpdateUserRequestBody>,
) -> ApiResult<Json<User>> {
    let input = UpdateUserInput::try_from(request_body)?;
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    let use_case = UserUseCase::new(user_repo, token_repo);
    let user = use_case
        .update(user.0.id, input)
        .await
        .map_err(internal_server_error)?;
    Ok(Json(user))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokensRequestBody {
    #[serde(serialize_with = "serialize_secret_string")]
    #[serde(deserialize_with = "deserialize_secret_string")]
    pub refresh_token: SecretString,
}

/// リフレッシュトークンハンドラ
#[tracing::instrument(skip(app_state))]
pub async fn refresh_tokens(
    cookie_jar: CookieJar,
    State(app_state): State<AppState>,
    request_body: Option<Json<RefreshTokensRequestBody>>,
) -> ApiResult<Response<Body>> {
    let requested_at = OffsetDateTime::now_utc();
    // クッキーからリフレッシュトークンを取得
    let mut refresh_token: Option<SecretString> = None;
    if let Some(cookie_value) = cookie_jar.get(COOKIE_REFRESH_TOKEN_KEY) {
        tracing::debug!("Found a refresh token in cookie");
        refresh_token = Some(SecretString::new(cookie_value.value().into()));
    }
    // リクエストボディからリフレッシュトークンを取得
    if refresh_token.is_none() && request_body.is_some() {
        tracing::debug!("Found a refresh token in body");
        refresh_token = Some(request_body.unwrap().0.refresh_token);
    }
    // リフレッシュトークンが見つからない場合は、401 Unauthorizedを返す
    let refresh_token = refresh_token.ok_or_else(unauthorized)?;
    // トークンリポジトリからリフレッシュトークンをキーに認証情報を取得
    let settings = &app_state.app_settings;
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    let token_key = generate_auth_token_info_key(&refresh_token);
    let token_content = token_repo
        .get_token_content(&token_key)
        .await
        .map_err(internal_server_error)?
        .ok_or_else(unauthorized)?;
    if token_content.token_type != TokenType::Refresh {
        return Err(bad_request("Invalid refresh token".into()));
    }
    // ユーザーリポジトリからユーザーを取得
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let user = user_repo
        .by_id(token_content.user_id)
        .await
        .map_err(internal_server_error)?;
    let user = user.ok_or_else(unauthorized)?;
    // ユーザーがロックされている場合は、423 Lockedを返す
    if !user.active {
        return Err(user_locked());
    }
    // アクセストークンとリフレッシュトークンを含めたレスポンスを返す
    generate_tokens_response(settings, user_repo, token_repo, user.id, requested_at).await
}

/// ログアウトハンドラ
#[tracing::instrument(skip(app_state))]
pub async fn logout(
    State(app_state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
) -> ApiResult<Response<Body>> {
    // ユーザーリポジトリからユーザーのハッシュ化されたアクセストークンとリフレッシュトークンを削除
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_keys = user_repo
        .delete_user_tokens_by_id(user.0.id)
        .await
        .map_err(internal_server_error)?;
    // トークンリポジトリから認証情報を削除
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    for key in token_keys.iter() {
        token_repo.delete_token_content(key).await?;
    }
    // レスポンスを作成
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response.headers_mut().insert(
        header::SET_COOKIE,
        Cookie::build((COOKIE_ACCESS_TOKEN_KEY, ""))
            .domain(&app_state.app_settings.http.host)
            .path("/")
            .http_only(true)
            .secure(app_state.app_settings.http.protocol == HttpProtocol::Https)
            .same_site(SameSite::Strict)
            .max_age(Duration::ZERO)
            .build()
            .to_string()
            .parse::<HeaderValue>()
            .unwrap(),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        Cookie::build((COOKIE_REFRESH_TOKEN_KEY, ""))
            .domain(&app_state.app_settings.http.host)
            .path("/")
            .http_only(true)
            .secure(app_state.app_settings.http.protocol == HttpProtocol::Https)
            .same_site(SameSite::Strict)
            .max_age(Duration::ZERO)
            .build()
            .to_string()
            .parse::<HeaderValue>()
            .unwrap(),
    );
    Ok(response)
}

async fn generate_tokens_response(
    settings: &AppSettings,
    user_repo: PgUserRepository,
    token_repo: RedisTokenRepository,
    user_id: UserId,
    requested_at: OffsetDateTime,
) -> ApiResult<Response<Body>> {
    // アクセストークンとリフレッシュトークンを生成
    let access_expired_at = requested_at + Duration::seconds(settings.token.access_max_age);
    let refresh_expired_at = requested_at + Duration::seconds(settings.token.refresh_max_age);
    let token_pair = generate_token_pair(
        user_id,
        access_expired_at,
        refresh_expired_at,
        &settings.token.jwt_secret,
    )?;
    // トークンリポジトリに認証情報を登録
    let access_token_info = generate_auth_token_info(
        user_id,
        &token_pair.access.0,
        TokenType::Access,
        settings.token.access_max_age as u64,
    );
    let refresh_token_info = generate_auth_token_info(
        user_id,
        &token_pair.refresh.0,
        TokenType::Refresh,
        settings.token.refresh_max_age as u64,
    );
    token_repo
        .register_token_pair(&access_token_info, &refresh_token_info)
        .await
        .map_err(internal_server_error)?;
    // ユーザーの最終ログイン日時を更新して、認証情報を登録するとともに、ログイン失敗履歴を削除
    user_repo
        .handle_logged_in(
            user_id,
            requested_at,
            &access_token_info.key,
            access_expired_at,
            &refresh_token_info.key,
            refresh_expired_at,
        )
        .await
        .map_err(internal_server_error)?;
    // レスポンスを作成
    let response_body = LoginResponseBody {
        access_token: token_pair.access.0,
        access_expired_at,
        refresh_token: token_pair.refresh.0,
        refresh_expired_at,
    };
    let mut response = Json(response_body.clone()).into_response();
    let access_cookie = create_cookie(
        settings.http.protocol,
        &settings.http.host,
        COOKIE_ACCESS_TOKEN_KEY,
        &response_body.access_token,
        Duration::seconds(settings.token.access_max_age),
    );
    let refresh_cookie = create_cookie(
        settings.http.protocol,
        &settings.http.host,
        COOKIE_REFRESH_TOKEN_KEY,
        &response_body.refresh_token,
        Duration::seconds(settings.token.refresh_max_age),
    );
    response.headers_mut().insert(
        header::SET_COOKIE,
        access_cookie.to_string().parse::<HeaderValue>().unwrap(),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        refresh_cookie.to_string().parse::<HeaderValue>().unwrap(),
    );
    Ok(response)
}

async fn handle_password_unmatched(
    settings: &AppSettings,
    user_repo: PgUserRepository,
    user_id: UserId,
    requested_at: OffsetDateTime,
) -> ApiResult<Response<Body>> {
    // ユーザーのログイン失敗履歴を取得
    match user_repo.get_login_failed_history(user_id).await? {
        None => {
            // ユーザーのログイン失敗履歴が存在しない場合は登録
            user_repo
                .create_login_failure_history(user_id, 1, requested_at)
                .await?;
        }
        Some(history) => {
            // ユーザーのログイン失敗履歴が存在する場合
            if requested_at - history.attempted_at
                < Duration::seconds(settings.login.attempts_seconds)
            {
                /*
                ログインを試行した日時から最初にログインに失敗した日時までの経過時間が、連続ログイン試行許容時間未満の場合、
                ログイン試行回数を1回増やす。その後、新しいログイン試行回数が、連続ログイン試行許容回数を超えば場合は、
                ユーザーのアクティブフラグを無効にする。
                 */
                user_repo
                    .increment_number_of_login_attempts(user_id, settings.login.max_attempts)
                    .await?;
            } else {
                /*
                ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間以上の場合、最初にログインを
                試行した日時をログインを試行した日時に更新して、連続ログイン試行回数を1に設定する。
                 */
                user_repo
                    .reset_login_failed_history(user_id, requested_at)
                    .await?;
            }
        }
    }
    Err(login_failed())
}

fn create_cookie<'c, N>(
    protocol: HttpProtocol,
    domain: &'c str,
    name: N,
    value: &'c SecretString,
    max_age: Duration,
) -> Cookie<'c>
where
    N: Into<Cow<'c, str>>,
{
    let cookie = Cookie::build((name.into(), value.expose_secret()))
        .domain(domain)
        .path("/")
        .http_only(true)
        .secure(protocol == HttpProtocol::Https)
        .same_site(SameSite::Strict)
        .max_age(max_age);
    cookie.build()
}
