use axum::{
    RequestExt as _,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse as _, Response},
};
use axum_extra::{
    TypedHeader,
    extract::cookie::CookieJar,
    headers::{Authorization, authorization::Bearer},
};
use secrecy::SecretString;

use domain::repositories::{
    TokenRepository as _, TokenType, UserRepository as _, generate_auth_token_info_key,
};
use use_case::AuthorizedUser;

use crate::{
    AppState,
    http::{ApiError, COOKIE_ACCESS_TOKEN_KEY, internal_server_error},
    postgres::repositories::PgUserRepository,
    redis::token::RedisTokenRepository,
};

/// HTTPリクエストヘッダーからアクセストークンを取り出し、アクセストークンの有効性を確認するミドルウェア
///
/// アクセストークンは、クッキー、またはAuthorizationヘッダーのBearerトークンとして提供される。
/// クッキーにアクセストークンが登録されている場合は、クッキーのアクセストークンを検証する。
/// クッキーにアクセストークンが登録されていない場合は、AuthorizationヘッダーのBearerトークンを検証する。
/// したがって、アクセストークンは、クッキーが優先される。
pub async fn authorized_user_middleware(
    State(app_state): State<AppState>,
    cookie_jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Response {
    // クッキーまたはAuthorizationヘッダーからトークンを取得
    let token = match get_access_token_from_request(&cookie_jar, &mut request).await {
        Some(token) => token,
        None => {
            // トークンが見つからない場合は、401 Unauthorizedを返す
            return ApiError {
                status_code: StatusCode::UNAUTHORIZED,
                messages: vec!["Access token is missing".into()],
            }
            .into_response();
        }
    };
    // トークンリポジトリからトークンをキーにトークンコンテンツを取得
    let token_repository = RedisTokenRepository::new(app_state.redis_pool);
    let key = generate_auth_token_info_key(&token);
    let token_content = match token_repository.get_token_content(&key).await {
        Ok(content) => content,
        Err(e) => {
            // トークンコンテンツを取得するときにエラーが発生した場合は、500 Internal Server Errorを返す
            return internal_server_error(e).into_response();
        }
    };
    // トークンコンテンツを取得できなかった場合は、トークンの有効期限が切れているか、無効なトークンであるため、
    // 401 Unauthorizedを返す
    if token_content.is_none() {
        return ApiError {
            status_code: StatusCode::UNAUTHORIZED,
            messages: vec!["Invalid or expired access token".into()],
        }
        .into_response();
    }
    let token_content = token_content.unwrap();
    // トークンコンテンツからアクセストークン（とみなしているトークン）が、本当にアクセストークンか確認して、
    // もしアクセストークンでなければ、400 Bad Requestを返す
    // トークンコンテンツは、アクセストークンであればTokenType::Access、リフレッシュトークンであればTokenType::Refreshを持つ
    if token_content.token_type != TokenType::Access {
        return ApiError {
            status_code: StatusCode::BAD_REQUEST,
            messages: vec!["Invalid access token".into()],
        }
        .into_response();
    }
    // アクセストークンが有効であるため、ユーザーを取得
    let user_repository = PgUserRepository::new(app_state.pg_pool);
    let user = user_repository.by_id(token_content.user_id).await;
    // ユーザーを取得するときにエラーが発生した場合は、500 Internal Server Errorを返す
    if user.is_err() {
        return internal_server_error(user.err().unwrap()).into_response();
    }
    let user = user.unwrap();
    // ユーザーが存在しない場合は、404 Not Foundを返す
    if user.is_none() {
        return ApiError {
            status_code: StatusCode::NOT_FOUND,
            messages: vec!["User not found".into()],
        }
        .into_response();
    }
    // 認証済みユーザーであることが確認できたため、リクエストにユーザー登録
    request
        .extensions_mut()
        .insert(AuthorizedUser(user.unwrap()));
    next.run(request).await
}

async fn get_access_token_from_request(
    cookie_jar: &CookieJar,
    request: &mut Request,
) -> Option<SecretString> {
    // クッキーからアクセストークンを取得
    tracing::debug!("Extracting access token from cookie...");
    if let Some(cookie_value) = cookie_jar.get(COOKIE_ACCESS_TOKEN_KEY) {
        tracing::debug!("Found a access token");
        return Some(SecretString::new(cookie_value.value().into()));
    }
    // Authorizationヘッダーからアクセストークンを取得
    let bearer = request
        .extract_parts::<TypedHeader<Authorization<Bearer>>>()
        .await;
    match bearer {
        Ok(bearer) => Some(SecretString::new(bearer.token().into())),
        Err(_) => None,
    }
}
