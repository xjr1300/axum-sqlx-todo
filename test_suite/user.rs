use std::collections::HashMap;

use cookie::{Cookie, SameSite};
use reqwest::StatusCode;
use secrecy::SecretString;
use settings::HttpProtocol;
use sqlx::types::time::OffsetDateTime;

use domain::{
    models::{USER_ROLE_CODE, User},
    repositories::TokenType,
};
use infra::http::{COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY};

use crate::{
    helpers::{ResponseParts, split_response},
    test_case::{
        REQUEST_TIMEOUT, RawLoginRequestBody, RawLoginResponseBody, RawSignUpRequestBody, TestCase,
    },
};

/// Ensure that a user can register, log in, and retrieve their information
#[tokio::test]
async fn integration_register_user_and_login_and_me() {
    let test_case = TestCase::begin(false).await;

    // Register a new user
    let sign_up_requested_at = OffsetDateTime::now_utc();
    let sign_up_request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&sign_up_request_body).await;
    let user: User = response.json().await.unwrap();
    assert_eq!(user.family_name.0, sign_up_request_body.family_name);
    assert_eq!(user.given_name.0, sign_up_request_body.given_name);
    assert_eq!(user.email.0, sign_up_request_body.email);
    assert_eq!(user.role.code.0, USER_ROLE_CODE);
    assert!(user.active);
    assert!(user.last_login_at.is_none());
    assert!((user.created_at - sign_up_requested_at).abs() < REQUEST_TIMEOUT);
    assert!((user.updated_at - sign_up_requested_at).abs() < REQUEST_TIMEOUT);
    let sign_up_user_id = user.id;

    // Log in with the new user
    let login_requested_at = OffsetDateTime::now_utc();
    let request_body: RawLoginRequestBody = sign_up_request_body.clone().into();
    let response = test_case.login(&request_body).await;
    let ResponseParts {
        status_code,
        headers,
        body,
    } = split_response(response).await;
    assert!(status_code.is_success());
    // Check to ensure that logged in user information was updated
    let user = test_case.user_by_id(user.id).await.unwrap();
    assert!((user.last_login_at.unwrap() - login_requested_at).abs() < REQUEST_TIMEOUT);
    assert!((user.updated_at - login_requested_at).abs() < REQUEST_TIMEOUT);
    // Check to ensure that the response body contains access and refresh tokens
    let login_response_body = serde_json::from_str::<RawLoginResponseBody>(&body).unwrap();
    let access_token = SecretString::new(login_response_body.access_token.clone().into());
    let access_content = test_case
        .token_content_by_token(&access_token)
        .await
        .unwrap();
    assert_eq!(access_content.user_id, sign_up_user_id);
    assert_eq!(access_content.token_type, TokenType::Access);
    let refresh_token = SecretString::new(login_response_body.refresh_token.clone().into());
    let refresh_content = test_case
        .token_content_by_token(&refresh_token)
        .await
        .unwrap();
    assert_eq!(refresh_content.user_id, sign_up_user_id);
    assert_eq!(refresh_content.token_type, TokenType::Refresh);
    // Check to ensure that access and refresh tokens are set in cookies
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

    // Retrieve the user information
    let response = test_case.me().await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK);
    let user: User = serde_json::from_str(&body).unwrap();
    assert_eq!(user.family_name.0, sign_up_request_body.family_name);
    assert_eq!(user.given_name.0, sign_up_request_body.given_name);
    assert_eq!(user.email.0, sign_up_request_body.email);
    assert!(user.active);
    assert!((user.last_login_at.unwrap() - login_requested_at).abs() < REQUEST_TIMEOUT);
    assert!((login_requested_at - user.created_at).abs() < REQUEST_TIMEOUT);
    assert!((user.updated_at - login_requested_at).abs() < REQUEST_TIMEOUT);

    test_case.end().await;
}

fn create_sign_up_request_body() -> RawSignUpRequestBody {
    RawSignUpRequestBody {
        family_name: String::from("Doe"),
        given_name: String::from("John"),
        email: String::from("john@example.com"),
        password: String::from("ab12$%AB"),
    }
}

/// Inspect that the cookie specification for access/refresh tokens is correct
///
/// # 引数
///
/// * `cookie` - cookie that contains the access/refresh token
/// * `expected_same_site` - expected `SameSite` attribute
/// * `expected_secure` - expected `Secure` attribute`
/// * `expected_http_only` - expected `HttpOnly` attribute
/// * `expected_max_age` - expected `MaxAge` for access/refresh token
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
