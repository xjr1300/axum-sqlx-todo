use axum::{
    RequestExt as _,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse as _, Response},
};
use axum_extra::{
    TypedHeader,
    extract::cookie::CookieJar,
    headers::{Authorization, authorization::Bearer},
};
use secrecy::SecretString;

use domain::repositories::{TokenRepository as _, TokenType, UserRepository as _};
use use_case::RequestUser;

use crate::{
    AppState,
    http::{ApiResult, COOKIE_ACCESS_TOKEN_KEY, bad_request, internal_server_error},
    postgres::repositories::PgUserRepository,
    redis::token::RedisTokenRepository,
};

/// HTTPリクエストヘッダーからアクセストークンを取り出し、アクセストークンの有効性を確認するミドルウェア
///
/// アクセストークンは、クッキー、またはAuthorizationヘッダーのBearerトークンとして提供される。
/// クッキーにアクセストークンが登録されている場合は、クッキーのアクセストークンを検証する。
/// クッキーにアクセストークンが登録されていない場合は、AuthorizationヘッダーのBearerトークンを検証する。
/// したがって、アクセストークンは、クッキーが優先される。
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Response {
    // クッキーにアクセストークンが存在するか確認
    let access_token_cookie = jar.get(COOKIE_ACCESS_TOKEN_KEY);
    if let Some(cookie) = access_token_cookie {
        // クッキーにアクセストークンが存在する場合は、クッキーのアクセストークンを検証
        let token = SecretString::new(cookie.value().into());
        if let Err(e) = verify_token(app_state, &token, &mut request).await {
            return e.into_response();
        }
        // クッキーのアクセストークンを検証したので、次のミドルウェアへ進む
        return next.run(request).await;
    }

    // クッキーにアクセストークンが登録されていないため、AuthorizationヘッダーのBearerトークンを検証
    let bearer = request
        .extract_parts::<TypedHeader<Authorization<Bearer>>>()
        .await;
    match bearer {
        Ok(bearer) => {
            let token = SecretString::new(bearer.token().into());
            if let Err(e) = verify_token(app_state, &token, &mut request).await {
                return e.into_response();
            }
        }
        Err(_) => {
            request.extensions_mut().insert(RequestUser::Anonymous);
        }
    }
    next.run(request).await
}

async fn verify_token(
    app_state: AppState,
    token: &SecretString,
    request: &mut Request,
) -> ApiResult<()> {
    let token_repository = RedisTokenRepository::new(app_state.redis_pool);
    let token_content = token_repository.retrieve_token_content(token).await;
    match token_content {
        Ok(Some(content)) => {
            // トークンリポジトリからトークンをキーにコンテンツを得られた場合
            match content.token_type {
                TokenType::Access => {
                    // アクセストークンの場合、ユーザーIDからユーザーを取得
                    let user_repository = PgUserRepository::new(app_state.pg_pool);
                    let user = user_repository
                        .by_id(content.user_id)
                        .await
                        .map_err(internal_server_error)?;
                    match user {
                        Some(user) => {
                            // ユーザーが存在する場合、リクエストの拡張にユーザーを追加
                            request.extensions_mut().insert(RequestUser::AuthUser(user));
                        }
                        None => {
                            // ユーザーが存在しない場合は、ユーザーを削除された可能性があるため、匿名ユーザーとして扱う
                            request.extensions_mut().insert(RequestUser::Anonymous);
                        }
                    }
                }
                TokenType::Refresh => {
                    // リフレッシュトークンの場合、エラーを返す
                    return Err(bad_request(
                        "a refresh token cannot be used for a bearer token".into(),
                    ));
                }
            }
        }
        Ok(None) => {
            // トークンリポジトリからトークンをキーにコンテンツが得られなかった場合は、トークンの有効期限が切れているか
            // 無効なトークン
            request.extensions_mut().insert(RequestUser::Anonymous);
        }
        Err(e) => return Err(internal_server_error(e)),
    }
    Ok(())
}
