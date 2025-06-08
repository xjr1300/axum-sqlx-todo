use std::collections::HashMap;

use cookie::{Cookie, SameSite};
use reqwest::StatusCode;
use secrecy::SecretString;
use settings::HttpProtocol;
use sqlx::types::time::OffsetDateTime;
use time::Duration;

use domain::{
    models::{USER_ROLE_CODE, User},
    repositories::TokenType,
};
use infra::http::{COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY};

use crate::{
    helpers::{ResponseParts, load_app_settings_for_testing, split_response},
    test_case::{
        REQUEST_TIMEOUT, RawLoginRequestBody, RawLoginResponseBody, RawSignUpRequestBody, TestCase,
    },
};

/// Ensure that a user can register, log in, and retrieve their information
#[tokio::test]
#[ignore]
async fn register_user_and_login_and_me() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, false).await;
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

    // TODO: Check that the access and refresh tokens are stored in postgres
    // TODO: Check that the access and refresh tokens are stored in redis

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

/// Inspect that the cookie specification for access/refresh tokens is correct
///
/// # Arguments
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

/// Ensure that entering an incorrect email address or password when logging in returns an error.
/// And ensure that the login failed history is recorded correctly if the email address is correct but the password is incorrect.
#[tokio::test]
#[ignore]
async fn login_with_invalid_credentials() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, false).await;
    let request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&request_body).await;
    let user: User = response.json().await.unwrap();
    let request_body: RawLoginRequestBody = request_body.clone().into();

    // Login with an wrong email address
    let wrong_email = RawLoginRequestBody {
        email: String::from("wrong@example.com"),
        ..request_body.clone()
    };
    let response = test_case.login(&wrong_email).await;
    let history = test_case.get_login_failed_history(user.id).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // No login failed history should be recorded if the email address is incorrect
    assert!(history.is_none());

    // Login with an wrong password
    let attempted_at = OffsetDateTime::now_utc();
    let wrong_password = RawLoginRequestBody {
        password: String::from("Wr0ng_password"),
        ..request_body.clone()
    };
    let response = test_case.login(&wrong_password).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // A login failed history should be recorded if the email address is correct but the password is incorrect
    assert_eq!(history.number_of_attempts, 1);
    assert!((history.attempted_at - attempted_at).abs() < REQUEST_TIMEOUT);
    test_case.end().await;
}

/// Ensure that the user are not locked even if the user fail to log in the maximum number of times allowed within the allowed time.
#[tokio::test]
#[ignore]
async fn user_not_locked_after_max_login_attempts() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, false).await;
    let request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&request_body).await;
    let user: User = response.json().await.unwrap();
    let correct_credential: RawLoginRequestBody = request_body.into();
    let incorrect_credential = RawLoginRequestBody {
        email: correct_credential.email.clone(),
        password: String::from("ab13$%AB"),
    };

    // Attempt to log in with an incorrect password multiple times
    for times in 0..test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(&incorrect_credential).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Check that the user is still active
    let user = test_case.user_by_id(user.id).await.unwrap();
    assert!(
        user.active,
        "User should not be locked after max login attempts"
    );
    // The user log in successfully , if attempt to log in with the correct password
    let response = test_case.login(&correct_credential).await;
    assert_eq!(response.status(), StatusCode::OK);
    test_case.end().await;
}

/// Ensure that the user was locked after exceeding the maximum number of login attempts within the allowed time
#[tokio::test]
#[ignore]
async fn user_locked_after_exceeding_max_login_attempts() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, false).await;
    let request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&request_body).await;
    let user: User = response.json().await.unwrap();
    let correct_credential: RawLoginRequestBody = request_body.into();
    let incorrect_credential = RawLoginRequestBody {
        email: correct_credential.email.clone(),
        password: String::from("ab13$%AB"),
    };

    // Attempt to log in with an incorrect password multiple times
    for times in 0..=test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(&incorrect_credential).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Check that the user is locked
    let user = test_case.user_by_id(user.id).await.unwrap();
    assert!(
        !user.active,
        "User should be locked after exceeding max login attempts"
    );
    // The user log in failed , if attempt to log in with the correct password
    let response = test_case.login(&correct_credential).await;
    assert_eq!(response.status(), StatusCode::LOCKED);
    test_case.end().await;
}

/// Ensure that the user can log in if, after the maximum login attempt time,
/// the user has failed to log in the maximum number of times within the allowed time.
#[tokio::test]
#[ignore]
async fn user_can_login_after_max_login_attempt_times() {
    let mut app_settings = load_app_settings_for_testing();
    // Set the maximum login attempts times to 1 and the maximum login attempts seconds to 1
    app_settings.login.max_attempts = 1;
    app_settings.login.attempts_seconds = 1;
    let test_case = TestCase::begin(app_settings, false).await;
    let request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&request_body).await;
    let user: User = response.json().await.unwrap();
    let correct_credential: RawLoginRequestBody = request_body.into();
    let incorrect_credential = RawLoginRequestBody {
        email: correct_credential.email.clone(),
        password: String::from("ab13$%AB"),
    };

    // Attempt to log in with an incorrect password
    let response = test_case.login(&incorrect_credential).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(history.number_of_attempts, 1);
    // Wait for the maximum login attempts seconds
    std::thread::sleep(std::time::Duration::from_secs(2));

    // The user log in successful
    let response = test_case.login(&correct_credential).await;
    assert_eq!(response.status(), StatusCode::OK);
    test_case.end().await;
}

/// Ensure that the user's login attempts are reset after the maximum login attempt time
#[tokio::test]
#[ignore]
async fn user_login_attempts_is_reset_after_max_login_attempt_time() {
    let mut app_settings = load_app_settings_for_testing();
    app_settings.login.max_attempts = 2;
    app_settings.login.attempts_seconds = 2;
    let test_case = TestCase::begin(app_settings, false).await;
    let request_body = create_sign_up_request_body();
    let response = test_case.sign_up(&request_body).await;
    let user: User = response.json().await.unwrap();
    let correct_credential: RawLoginRequestBody = request_body.into();
    let incorrect_credential = RawLoginRequestBody {
        email: correct_credential.email.clone(),
        password: String::from("ab13$%AB"),
    };

    // Attempt to log in with an incorrect password
    for times in 0..test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(&incorrect_credential).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Wait for the maximum login attempts seconds
    std::thread::sleep(std::time::Duration::from_secs(3));

    // The user log in failed
    let requested_at = OffsetDateTime::now_utc();
    let response = test_case.login(&incorrect_credential).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let duration = history.attempted_at - requested_at;
    assert!(duration < Duration::seconds(1));
    assert_eq!(history.number_of_attempts, 1);
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
