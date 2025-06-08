use std::borrow::Cow;

use axum::{
    Extension, Json,
    body::Body,
    extract::State,
    http::{HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use cookie::{Cookie, SameSite};
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use settings::{HttpProtocol, HttpSettings, TokenSettings};
use time::{Duration, OffsetDateTime, serde::rfc3339};

use domain::{
    DomainError, DomainResult,
    models::{Email, FamilyName, GivenName, RawPassword, User, UserId},
    repositories::{TokenPairWithExpired, UserInput, UserRepository},
};
use use_case::{AuthorizedUser, user::UserUseCase};
use utils::serde::serialize_secret_string;

use crate::{
    AppState,
    http::{
        ApiError, ApiResult, COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY, bad_request,
        internal_server_error,
    },
    jwt::generate_token_pair,
    password::{create_hashed_password, verify_password},
    postgres::repositories::PgUserRepository,
    redis::token::RedisTokenRepository,
};

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
    access_expiration: OffsetDateTime,
    #[serde(serialize_with = "serialize_secret_string")]
    refresh_token: SecretString,
    #[serde(serialize_with = "rfc3339::serialize")]
    refresh_expiration: OffsetDateTime,
}

/// ログインハンドラ
///
/// # 手順
///
/// ## 1. ユーザーの取得
///
/// ユーザーのEメールアドレスから、リポジトリからユーザーを取得する。
/// リポジトリからユーザーを取得できなかった場合は、404 Not Foundエラーを返す。
///
/// ## 2. ユーザーのアクティブフラグの確認
///
/// ユーザーのアクティブフラグを確認して、アクティブでない場合は、403 Forbiddenエラーを返す。
///
/// ## 3. パスワードの確認
///
/// 次に、ユーザーのパスワードを検証する。
///
/// ### 3.1. パスワードが一致した場合
///
/// ユーザーのパスワードが一致した場合は、最終ログイン日時を更新して、ログイン失敗履歴から、
/// そのユーザーのレコードを削除する。
/// その後、200 OKレスポンスとユーザー情報を返す。
///
/// ### 3.2. パスワードが一致しない場合
///
/// ユーザーのログイン試行履歴を確認して、次の通り処理する。
/// その後、401 Unauthorizedエラーを返す。
///
/// #### 3.2.1. ログイン試行履歴が存在しない場合
///
/// ログイン失敗履歴にそのユーザーのレコードが存在しない場合は、
/// ログイン失敗履歴に次のレコードを追加する。
///
/// - ログイン失敗回数: 1
/// - ログイン試行開始日時: 現在の日時
///
///
/// #### 3.2.2. ログイン試行履歴が存在する場合
///
/// ログイン失敗履歴にそのユーザーのレコードが存在する場合は、ログイン失敗履歴のログイン試行開始日時を確認する。
///
/// ##### 3.2.2.1. ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間未満の場合
///
/// ```text
/// 現在日時 - ログイン試行開始日時 < 連続ログイン試行許容時間
/// ```
///
/// ログイン失敗履歴のログイン試行回数を1回増やす。
/// そしてログイン試行回数が連続ログイン試行許容回数を超えた場合は、ユーザーのアクティブフラグを無効にする。
///
/// ##### 3.2.2.2. ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間以上の場合
///
/// ```text
/// 現在日時 - ログイン試行開始日時 >= 連続ログイン試行許容時間
/// ```
///
/// ログイン試行履歴のログイン試行回数を1に、ログイン試行開始日時を現在の日時に更新する。
#[tracing::instrument(skip(app_state))]
pub async fn login(
    State(app_state): State<AppState>,
    Json(request_body): Json<LoginRequestBody>,
) -> ApiResult<Response<Body>> {
    let attempted_at = OffsetDateTime::now_utc();
    let settings = &app_state.app_settings;
    let use_case = user_use_case(&app_state);

    // Eメールアドレスからユーザーを取得
    let email =
        Email::new(request_body.email).map_err(|_| bad_request("Invalid email address".into()))?;
    let user = use_case
        .user_repository
        .by_email(&email)
        .await
        .map_err(internal_server_error)?
        .ok_or_else(login_failed_response)?;
    // ユーザーのアクティブフラグを確認
    if !user.active {
        return Err(ApiError {
            status_code: StatusCode::LOCKED,
            messages: vec![USER_LOCKED.into()],
        });
    }
    // ユーザーのハッシュ化されたパスワードを取得
    let hashed_password = use_case
        .user_repository
        .get_hashed_password(user.id)
        .await
        .map_err(internal_server_error)?;
    // ユーザーのパスワードを検証
    let raw_password =
        RawPassword::new(request_body.password).map_err(|_| login_failed_response())?;
    if verify_password(&raw_password, &settings.password.pepper, &hashed_password)
        .map_err(internal_server_error)?
    {
        Ok(handle_login_succeed(
            &settings.http,
            &settings.token,
            use_case,
            user.id,
            attempted_at,
        )
        .await?)
    } else {
        use_case
            .handle_login_failure(
                user.id,
                attempted_at,
                settings.login.max_attempts,
                settings.login.attempts_seconds,
            )
            .await
            .map_err(internal_server_error)?;
        Err(login_failed_response())
    }
}

fn user_use_case(app_state: &AppState) -> UserUseCase<PgUserRepository, RedisTokenRepository> {
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    UserUseCase::new(user_repo, token_repo)
}

async fn store_login_info(
    token_settings: &TokenSettings,
    use_case: UserUseCase<PgUserRepository, RedisTokenRepository>,
    user_id: UserId,
    attempted_at: OffsetDateTime,
) -> ApiResult<LoginResponseBody> {
    // アクセストークンとリフレッシュトークンを作成
    let access_expired_at = attempted_at + Duration::seconds(token_settings.access_max_age);
    let refresh_expired_at = attempted_at + Duration::seconds(token_settings.refresh_max_age);
    let token_pair = generate_token_pair(
        user_id,
        access_expired_at,
        refresh_expired_at,
        &token_settings.jwt_secret,
    )?;
    // アクセストークンとリフレッシュトークンを、ハッシュ化してRedisに登録
    // Redisには、アクセストークンをハッシュ化した文字列をキーに、ユーザーIDとトークンの種類を表現する文字列を':'で
    // 連結した文字列を値に追加する。
    // Redisに登録するレコードは、トークンの種類別の有効期限を設定する。
    let token_pair_with_expired = TokenPairWithExpired {
        access: &token_pair.access.0,
        access_expired_at,
        refresh: &token_pair.refresh.0,
        refresh_expired_at,
    };
    use_case
        .store_login_info(
            user_id,
            token_pair_with_expired,
            token_settings.access_max_age,
            token_settings.refresh_max_age,
            attempted_at,
        )
        .await
        .map_err(internal_server_error)?;
    Ok(LoginResponseBody {
        access_token: token_pair.access.0,
        access_expiration: access_expired_at,
        refresh_token: token_pair.refresh.0,
        refresh_expiration: refresh_expired_at,
    })
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

async fn handle_login_succeed(
    http_settings: &HttpSettings,
    token_settings: &TokenSettings,
    use_case: UserUseCase<PgUserRepository, RedisTokenRepository>,
    user_id: UserId,
    attempted_at: OffsetDateTime,
) -> ApiResult<Response<Body>> {
    // ログイン情報を記録
    let response_body = store_login_info(token_settings, use_case, user_id, attempted_at).await?;
    // レスポンスを作成
    let mut response = Json(response_body.clone()).into_response();
    let access_cookie = create_cookie(
        http_settings.protocol,
        &http_settings.host,
        COOKIE_ACCESS_TOKEN_KEY,
        &response_body.access_token,
        Duration::seconds(token_settings.access_max_age),
    );
    let refresh_cookie = create_cookie(
        http_settings.protocol,
        &http_settings.host,
        COOKIE_REFRESH_TOKEN_KEY,
        &response_body.refresh_token,
        Duration::seconds(token_settings.refresh_max_age),
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

const LOGIN_FAILED: &str = "Login failed. Please check your email and password";
const USER_LOCKED: &str = "User is locked";

fn login_failed_response() -> ApiError {
    ApiError {
        status_code: StatusCode::UNAUTHORIZED,
        messages: vec![LOGIN_FAILED.into()],
    }
}

#[tracing::instrument]
pub async fn me(Extension(user): Extension<AuthorizedUser>) -> ApiResult<Json<User>> {
    Ok(Json(user.0))
}
