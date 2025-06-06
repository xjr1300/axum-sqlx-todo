use std::{collections::HashMap, str::FromStr as _};

use cookie::{Cookie, SameSite};
use secrecy::SecretString;
use settings::HttpProtocol;
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

use domain::{models::UserId, repositories::TokenType};
use infra::http::{
    COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY, handler::user::UserResponseBody,
};

use crate::helpers::{
    REQUEST_TIMEOUT, RawLoginRequestBody, RawLoginResponseBody, RawSignUpRequestBody,
    ResponseParts, TestCase, split_response,
};

/// Test case for user registration, login, and fetching user info
#[tokio::test]
async fn integration_register_user_and_login_and_me() {
    let test_case = TestCase::begin(false).await;

    // Register a new user
    let requested_at = OffsetDateTime::now_utc();
    let request_body = create_signup_request_body();
    let response = test_case.sign_up(&request_body).await;
    let response_body: UserResponseBody = response.json().await.unwrap();
    assert_eq!(response_body.family_name, request_body.family_name);
    assert_eq!(response_body.given_name, request_body.given_name);
    assert_eq!(response_body.email, request_body.email);
    assert!(response_body.active);
    assert!(response_body.last_login_at.is_none());
    assert!(response_body.created_at >= requested_at);
    assert!(response_body.updated_at >= requested_at);
    let user_id = UserId::from(Uuid::from_str(&response_body.id).unwrap());

    // Log in with the new user
    let requested_at = OffsetDateTime::now_utc();
    let request_body: RawLoginRequestBody = request_body.into();
    let response = test_case.login(&request_body).await;
    let ResponseParts {
        status_code,
        headers,
        body,
    } = split_response(response).await;
    assert!(status_code.is_success());
    let user = test_case.user_by_id(user_id).await.unwrap();
    assert!(user.last_login_at.unwrap() - requested_at < REQUEST_TIMEOUT);
    assert!(user.updated_at - requested_at < REQUEST_TIMEOUT);
    let login_response_body = serde_json::from_str::<RawLoginResponseBody>(&body).unwrap();
    let access_token = SecretString::new(login_response_body.access_token.clone().into());
    let access_content = test_case
        .token_content_by_token(&access_token)
        .await
        .unwrap();
    assert_eq!(access_content.user_id, user_id);
    assert_eq!(access_content.token_type, TokenType::Access);
    let refresh_token = SecretString::new(login_response_body.refresh_token.clone().into());
    let refresh_content = test_case
        .token_content_by_token(&refresh_token)
        .await
        .unwrap();
    assert_eq!(refresh_content.user_id, user_id);
    assert_eq!(refresh_content.token_type, TokenType::Refresh);
    // クッキーにアクセストークンとリフレッシュトークンが設定されていることを確認する
    let set_cookie_values = headers.get_all(reqwest::header::SET_COOKIE);
    let mut set_cookies: HashMap<String, Cookie> = HashMap::new();
    for value in set_cookie_values {
        let cookie = Cookie::parse(value.to_str().unwrap()).unwrap();
        set_cookies.insert(cookie.name().to_string(), cookie);
    }
    let access_cookie = set_cookies.get(COOKIE_ACCESS_TOKEN_KEY).unwrap();
    inspect_token_cookie_spec(
        access_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        test_case.app_state.app_settings.token.access_max_age,
    );
    let refresh_cookie = set_cookies.get(COOKIE_REFRESH_TOKEN_KEY).unwrap();
    inspect_token_cookie_spec(
        refresh_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        test_case.app_state.app_settings.token.refresh_max_age,
    );

    test_case.end().await;
}

fn create_signup_request_body() -> RawSignUpRequestBody {
    RawSignUpRequestBody {
        family_name: String::from("Doe"),
        given_name: String::from("John"),
        email: String::from("john@example.com"),
        password: String::from("ab12$%AB"),
    }
}

/// アクセス／リフレッシュトークン保存するクッキーの仕様を確認する。
///
/// # 引数
///
/// * `cookie` - アクセス／リフレッシュトークンを保存するクッキー
/// * `expected_same_site` - 予期する`SameSite`の値
/// * `expected_secure` - `Secure`を設定するかを示すフラグ
/// * `expected_http_only` - `HttpOnly`を設定するかを示すフラグ
/// * `expected_max_age` - アクセス／リフレッシュトークンの有効期限（秒）
fn inspect_token_cookie_spec(
    cookie: &Cookie<'_>,
    expected_same_site: SameSite,
    expected_secure: bool,
    expected_http_only: bool,
    expected_max_age: i64,
) {
    assert_eq!(
        expected_same_site.to_string(),
        cookie.same_site().unwrap().to_string()
    );
    if expected_secure {
        assert!(cookie.secure().is_some(), "Secure flag should be set");
    } else {
        assert!(cookie.secure().is_none(), "Secure flag should not be set");
    }
    assert_eq!(
        expected_http_only,
        cookie.http_only().unwrap(),
        "HttpOnly flag mismatch"
    );
    assert_eq!(
        expected_max_age,
        cookie.max_age().unwrap().whole_seconds(),
        "Cookie expiration mismatch"
    );
}
