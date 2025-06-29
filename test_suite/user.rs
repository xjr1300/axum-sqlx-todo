use std::{collections::HashMap, sync::Arc};

use cookie::{Cookie, SameSite};
use reqwest::{StatusCode, Url};
use secrecy::{ExposeSecret as _, SecretString};
use sqlx::types::time::OffsetDateTime;
use time::Duration;

use domain::{
    models::{RoleCode, User},
    repositories::{TokenType, generate_auth_token_info_key},
};
use infra::{
    http::{COOKIE_ACCESS_TOKEN_KEY, COOKIE_REFRESH_TOKEN_KEY},
    jwt::{Claim, generate_token},
    settings::HttpProtocol,
};

use crate::{
    helpers::{ResponseParts, load_app_settings_for_testing, split_response},
    test_case::{EnableTracing, InsertTestData, REQUEST_TIMEOUT, RawLoginResponseBody, TestCase},
};

/// Check that a user can register, log in, retrieve their information, and log out successfully.
#[tokio::test]
#[ignore]
async fn user_use_case_test() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    // Register a new user
    let sign_up_requested_at = OffsetDateTime::now_utc();
    let sign_up_request_body = create_sign_up_request_body();
    let response = test_case.sign_up(sign_up_request_body).await;
    let user: User = response.json().await.unwrap();
    assert_eq!(user.family_name.0, "Doe");
    assert_eq!(user.given_name.0, "John");
    assert_eq!(user.email.0, "john@example.com");
    assert_eq!(user.role.code, RoleCode::User);
    assert!(user.active);
    assert!(user.last_login_at.is_none());
    assert!((user.created_at - sign_up_requested_at).abs() < REQUEST_TIMEOUT);
    assert!((user.updated_at - sign_up_requested_at).abs() < REQUEST_TIMEOUT);
    let sign_up_user_id = user.id;

    // Log in with the new user
    let login_requested_at = OffsetDateTime::now_utc();
    let response = test_case.login(john_credentials()).await;
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
        .token_content_from_token_repo(&access_token)
        .await
        .unwrap();
    assert_eq!(access_content.user_id, sign_up_user_id);
    assert_eq!(access_content.token_type, TokenType::Access);
    let refresh_token = SecretString::new(login_response_body.refresh_token.clone().into());
    let refresh_content = test_case
        .token_content_from_token_repo(&refresh_token)
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
    assert_eq!(access_cookie.value(), access_token.expose_secret());
    inspect_token_cookie_spec(
        access_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        test_case.app_state.app_settings.token.access_max_age,
    );
    let refresh_cookie = set_cookies.get(COOKIE_REFRESH_TOKEN_KEY).unwrap();
    assert_eq!(refresh_cookie.value(), refresh_token.expose_secret());
    inspect_token_cookie_spec(
        refresh_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        test_case.app_state.app_settings.token.refresh_max_age,
    );

    // Check that the access and refresh tokens are stored in postgres
    let access_key = generate_auth_token_info_key(&access_token);
    let refresh_key = generate_auth_token_info_key(&refresh_token);
    let user_token_keys = test_case.user_tokens_from_user_repo(user.id).await;
    assert!(
        user_token_keys
            .iter()
            .any(|ut| ut.token_key.expose_secret() == access_key.expose_secret())
    );
    assert!(
        user_token_keys
            .iter()
            .any(|ut| ut.token_key.expose_secret() == refresh_key.expose_secret())
    );

    // Check that the access and refresh tokens are stored in redis
    assert!(
        test_case
            .token_content_from_token_repo(&access_token)
            .await
            .is_some()
    );
    assert!(
        test_case
            .token_content_from_token_repo(&refresh_token)
            .await
            .is_some()
    );

    // Retrieve the user information
    let response = test_case.me().await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK);
    let user: User = serde_json::from_str(&body).unwrap();
    assert_eq!(user.family_name.0, "Doe");
    assert_eq!(user.given_name.0, "John");
    assert_eq!(user.email.0, "john@example.com");
    assert!(user.active);
    assert!((user.last_login_at.unwrap() - login_requested_at).abs() < REQUEST_TIMEOUT);
    assert!((login_requested_at - user.created_at).abs() < REQUEST_TIMEOUT);
    assert!((user.updated_at - login_requested_at).abs() < REQUEST_TIMEOUT);

    // Logout
    let response = test_case.logout().await;
    let ResponseParts {
        status_code,
        headers,
        body,
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::NO_CONTENT);
    assert!(body.is_empty());

    // Check that the access and refresh tokens in cookies are deleted
    let set_cookie_values = headers.get_all(reqwest::header::SET_COOKIE);
    let mut set_cookies: HashMap<String, Cookie> = HashMap::new();
    for value in set_cookie_values {
        let cookie = Cookie::parse(value.to_str().unwrap()).unwrap();
        set_cookies.insert(cookie.name().to_string(), cookie);
    }
    let access_cookie = set_cookies.get(COOKIE_ACCESS_TOKEN_KEY).unwrap();
    assert_eq!(access_cookie.value(), "");
    inspect_token_cookie_spec(
        access_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        0,
    );
    let refresh_cookie = set_cookies.get(COOKIE_REFRESH_TOKEN_KEY).unwrap();
    assert_eq!(refresh_cookie.value(), "");
    inspect_token_cookie_spec(
        refresh_cookie,
        SameSite::Strict,
        test_case.app_state.app_settings.http.protocol == HttpProtocol::Https,
        true,
        0,
    );

    // Check that the access and refresh tokens are deleted from postgres
    assert!(
        test_case
            .user_tokens_from_user_repo(user.id)
            .await
            .is_empty(),
        "User tokens should be deleted after logout"
    );

    // Check that the access and refresh tokens are deleted from redis
    assert!(
        test_case
            .token_content_from_token_repo(&access_token)
            .await
            .is_none()
    );
    assert!(
        test_case
            .token_content_from_token_repo(&refresh_token)
            .await
            .is_none()
    );

    test_case.end().await;
}

/// Check that entering an incorrect email address or password when logging in returns an error.
/// And ensure that the login failed history is recorded correctly if the email address is correct but the password is incorrect.
#[tokio::test]
#[ignore]
async fn user_can_not_login_with_invalid_credentials() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;
    let (user, ..) = create_user_and_login(&test_case).await;

    // Login with an wrong email address
    let request_body = String::from(
        r#"
        {
            "email": "wrong@example.com",
            "password": "ab12$%AB"
        }
        "#,
    );
    let response = test_case.login(request_body).await;
    let history = test_case.get_login_failed_history(user.id).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // No login failed history should be recorded if the email address is incorrect
    assert!(history.is_none());

    // Login with an wrong password
    let attempted_at = OffsetDateTime::now_utc();
    let response = test_case.login(john_incorrect_credential()).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // A login failed history should be recorded if the email address is correct but the password is incorrect
    assert_eq!(history.number_of_attempts, 1);
    assert!((history.attempted_at - attempted_at).abs() < REQUEST_TIMEOUT);

    test_case.end().await;
}

/// Check that the user can not login when the user is locked.
#[tokio::test]
#[ignore]
async fn user_can_not_login_when_user_is_locked() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    test_case.set_user_active_status(user.id, false).await;

    let response = test_case.login(john_credentials()).await;
    assert_eq!(response.status(), StatusCode::LOCKED);

    test_case.end().await;
}

/// Check that the user are not locked even if the user fail to log in the maximum number of times allowed within the allowed time.
#[tokio::test]
#[ignore]
async fn user_is_not_locked_after_user_attempts_to_login_in_max_attempt_times() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    // Attempt to log in with an incorrect password multiple times
    for times in 0..test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(john_incorrect_credential()).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Check that the user is still active
    let user = test_case.user_by_id(user.id).await.unwrap();
    assert!(
        user.active,
        "User should not be locked after max login attempts"
    );
    // The user log in successfully , if attempt to log in with the correct password
    let response = test_case.login(john_credentials()).await;
    assert_eq!(response.status(), StatusCode::OK);

    test_case.end().await;
}

/// Check that the user was locked after exceeding the maximum number of login attempts within the allowed time
#[tokio::test]
#[ignore]
async fn user_is_locked_after_use_attempts_to_login_exceeding_max_attempts() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    // Attempt to log in with an incorrect password multiple times
    for times in 0..=test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(john_incorrect_credential()).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Check that the user is locked
    let user = test_case.user_by_id(user.id).await.unwrap();
    assert!(
        !user.active,
        "User should be locked after exceeding max login attempts"
    );
    // The user log in failed , if attempt to log in with the correct password
    let response = test_case.login(john_credentials()).await;
    assert_eq!(response.status(), StatusCode::LOCKED);

    test_case.end().await;
}

/// Check that the user can log in if, after the maximum login attempt time,
/// the user has failed to log in the maximum number of times within the allowed time.
#[tokio::test]
#[ignore]
async fn user_can_login_after_user_attempts_to_login_in_max_attempt_times() {
    let mut app_settings = load_app_settings_for_testing();
    // Set the maximum login attempts times to 1 and the maximum login attempts seconds to 1
    app_settings.login.max_attempts = 1;
    app_settings.login.attempts_seconds = 1;
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    // Attempt to log in with an incorrect password
    let response = test_case.login(john_incorrect_credential()).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(history.number_of_attempts, 1);
    // Wait for the maximum login attempts seconds
    std::thread::sleep(std::time::Duration::from_secs(2));

    // The user log in successful
    let response = test_case.login(john_credentials()).await;
    assert_eq!(response.status(), StatusCode::OK);

    test_case.end().await;
}

/// Check that the user's login attempts are reset after the maximum login attempt time
#[tokio::test]
#[ignore]
async fn user_login_failed_history_is_reset_after_max_attempt_time() {
    let mut app_settings = load_app_settings_for_testing();
    app_settings.login.max_attempts = 2;
    app_settings.login.attempts_seconds = 2;
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    // Attempt to log in with an incorrect password
    for times in 0..test_case.app_state.app_settings.login.max_attempts {
        let response = test_case.login(john_incorrect_credential()).await;
        let history = test_case.get_login_failed_history(user.id).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(history.number_of_attempts, times + 1);
    }
    // Wait for the maximum login attempts seconds
    std::thread::sleep(std::time::Duration::from_secs(3));

    // The user log in failed
    let requested_at = OffsetDateTime::now_utc();
    let response = test_case.login(john_incorrect_credential()).await;
    let history = test_case.get_login_failed_history(user.id).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let duration = history.attempted_at - requested_at;
    assert!(duration < Duration::seconds(1));
    assert_eq!(history.number_of_attempts, 1);

    test_case.end().await;
}

/// Check that the user who is locked can not get their information.
#[tokio::test]
#[ignore]
async fn user_can_not_get_user_information_when_user_is_locked() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    test_case.set_user_active_status(user.id, false).await;

    let response = test_case.me().await;
    assert_eq!(
        response.status(),
        StatusCode::LOCKED,
        "{}",
        response.text().await.unwrap()
    );

    test_case.end().await;
}

///
/// Check that an anonymous user can not access the user information endpoint.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_access_user_information_endpoint() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let response = test_case.me().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

/// Check that the user can update their information successfully.
#[tokio::test]
#[ignore]
async fn user_can_update_user_information_with_credentials() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, ..) = create_user_and_login(&test_case).await;
    let _ = test_case.login(john_credentials()).await;

    // Update user information
    let requested_at = OffsetDateTime::now_utc();
    let request_body = String::from(
        r#"
        {
            "familyName": "Smith",
            "givenName": "Jane",
            "email": "jane@example.com"

        }
        "#,
    );
    let response = test_case.update_user(request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK);
    let updated_user: User = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.family_name, "Smith");
    assert_eq!(updated_user.given_name, "Jane",);
    assert_eq!(updated_user.email, "jane@example.com");
    assert!((updated_user.updated_at - requested_at).abs() < REQUEST_TIMEOUT);

    // Update family name only
    let family_name_only = String::from(
        r#"
        {
            "familyName": "Schmo"
        }
        "#,
    );
    let response = test_case.update_user(family_name_only).await;
    let updated_user: User = response.json().await.unwrap();
    assert_eq!(updated_user.family_name, "Schmo");
    assert_eq!(updated_user.given_name, "Jane");
    assert_eq!(updated_user.email, "jane@example.com");

    // Update given name only
    let given_name_only = String::from(
        r#"
        {
            "givenName": "Alice"
        }
        "#,
    );
    let response = test_case.update_user(given_name_only).await;
    let updated_user: User = response.json().await.unwrap();
    assert_eq!(updated_user.family_name, "Schmo");
    assert_eq!(updated_user.given_name, "Alice");
    assert_eq!(updated_user.email, "jane@example.com");

    // Update email only
    let email_only = String::from(
        r#"
        {
            "email": "alice@example.com"
        }
        "#,
    );
    let response = test_case.update_user(email_only).await;
    let updated_user: User = response.json().await.unwrap();
    assert_eq!(updated_user.family_name, "Schmo");
    assert_eq!(updated_user.given_name, "Alice");
    assert_eq!(updated_user.email, "alice@example.com");

    test_case.end().await;
}

/// Check that an anonymous user can not access an user update endpoint.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_update_user_information() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let response = test_case.me().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

/// Check that the access and refresh tokens are refreshed when the user requests with a valid refresh token in the cookie.
#[tokio::test]
#[ignore]
async fn user_can_refresh_tokens_with_valid_refresh_token_in_the_cookie() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let _ = create_user_and_login(&test_case).await;
    let response = test_case.refresh_tokens().await;
    let ResponseParts {
        status_code,
        headers,
        body,
    } = split_response(response).await;
    assert!(status_code.is_success());
    let body: RawLoginResponseBody = serde_json::from_str(&body).unwrap();
    let set_cookie_values = headers.get_all(reqwest::header::SET_COOKIE);
    let mut set_cookies: HashMap<String, Cookie> = HashMap::new();
    for value in set_cookie_values {
        let cookie = Cookie::parse(value.to_str().unwrap()).unwrap();
        set_cookies.insert(cookie.name().to_string(), cookie);
    }
    let access_cookie = set_cookies.get(COOKIE_ACCESS_TOKEN_KEY).unwrap();
    assert_eq!(access_cookie.value(), body.access_token);
    let refresh_cookie = set_cookies.get(COOKIE_REFRESH_TOKEN_KEY).unwrap();
    assert_eq!(refresh_cookie.value(), body.refresh_token);

    test_case.end().await;
}

// Check that the access and refresh tokens are refreshed when the user requests with a valid refresh token in the body.
#[tokio::test]
#[ignore]
async fn user_can_refresh_tokens_with_valid_refresh_token_in_the_body() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (_, tokens) = create_user_and_login(&test_case).await;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let uri = format!("{}/users/refresh-tokens", test_case.origin());
    let body = format!(
        r#"
        {{
            "refreshToken": "{}"
        }}
        "#,
        tokens.refresh_token
    );
    let response = client
        .post(&uri)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    test_case.end().await;
}

/// Check that the access and refresh tokens are not refreshed when the request is missing the refresh token.
#[tokio::test]
#[ignore]
async fn user_can_not_refresh_tokens_without_refresh_token() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let _ = create_user_and_login(&test_case).await;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let uri = format!("{}/users/refresh-tokens", test_case.origin());
    let response = client
        .post(&uri)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    test_case.end().await;
}

/// Check that the access and refresh tokens are not refreshed when the user requests with an invalid refresh token in the cookie.
#[tokio::test]
#[ignore]
async fn user_can_not_refresh_tokens_invalid_refresh_token_in_the_cookie() {
    let app_settings = load_app_settings_for_testing();
    let test_case =
        TestCase::begin(app_settings.clone(), EnableTracing::No, InsertTestData::No).await;

    let (user, _) = create_user_and_login(&test_case).await;
    let claim = Claim {
        user_id: user.id,
        expiration: 3000,
    };
    let url = Url::parse(&format!(
        "{}:{}",
        app_settings.http.protocol, app_settings.http.host
    ))
    .unwrap();
    let refresh_token = generate_token(claim, &SecretString::new("secret-key".into())).unwrap();
    let cookie_jar = reqwest::cookie::Jar::default();
    cookie_jar.add_cookie_str(
        &format!(
            "{}={}",
            COOKIE_REFRESH_TOKEN_KEY,
            refresh_token.expose_secret()
        ),
        &url,
    );
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_provider(Arc::new(cookie_jar))
        .build()
        .unwrap();
    let uri = format!("{}/users/refresh-tokens", test_case.origin());
    let response = client.post(&uri).send().await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "{}",
        response.text().await.unwrap()
    );

    test_case.end().await;
}

/// Check that the access and refresh tokens are not refreshed when the user requests with an expired refresh token in the cookie.
#[tokio::test]
#[ignore]
async fn user_can_not_refresh_tokens_refresh_token_was_expired() {
    let mut app_settings = load_app_settings_for_testing();
    // Set the refresh token expiration to 1 second
    app_settings.token.refresh_max_age = 1;
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let _ = create_user_and_login(&test_case).await;
    // Wait for the refresh token to expire
    std::thread::sleep(std::time::Duration::from_secs_f32(1.5));
    let response = test_case.refresh_tokens().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

/// Check that the access and refresh tokens are not refreshed when the user requests with an access token in the cookie.
#[tokio::test]
#[ignore]
async fn user_can_not_refresh_tokens_with_access_token_in_the_cookie() {
    let app_settings = load_app_settings_for_testing();
    let test_case =
        TestCase::begin(app_settings.clone(), EnableTracing::No, InsertTestData::No).await;

    let (.., tokens) = create_user_and_login(&test_case).await;
    let url = Url::parse(&format!(
        "{}:{}",
        app_settings.http.protocol, app_settings.http.host
    ))
    .unwrap();
    let cookie_jar = reqwest::cookie::Jar::default();
    cookie_jar.add_cookie_str(
        &format!("{}={}", COOKIE_REFRESH_TOKEN_KEY, tokens.access_token),
        &url,
    );
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_provider(Arc::new(cookie_jar))
        .build()
        .unwrap();
    let uri = format!("{}/users/refresh-tokens", test_case.origin());
    let response = client.post(&uri).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);

    test_case.end().await;
}

/// Check that the access and refresh tokens are not refreshed when the user who requested them is locked.
#[tokio::test]
#[ignore]
async fn user_can_not_refresh_tokens_when_the_user_is_locked() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let (user, _) = create_user_and_login(&test_case).await;
    test_case.set_user_active_status(user.id, false).await;
    let response = test_case.refresh_tokens().await;
    assert_eq!(response.status(), StatusCode::LOCKED);

    test_case.end().await;
}

fn john_credentials() -> String {
    String::from(
        r#"
        {
            "email": "john@example.com",
            "password": "ab12$%AB"
        }
        "#,
    )
}

fn john_incorrect_credential() -> String {
    String::from(
        r#"
        {
            "email": "john@example.com",
            "password": "ab13$%AB"
        }
        "#,
    )
}

fn create_sign_up_request_body() -> String {
    String::from(
        r#"
        {
            "familyName": "Doe",
            "givenName": "John",
            "email": "john@example.com",
            "password": "ab12$%AB"
        }
        "#,
    )
}

async fn create_user_and_login(test_case: &TestCase) -> (User, RawLoginResponseBody) {
    let body = create_sign_up_request_body();
    let response = test_case.sign_up(body).await;
    let user: User = response.json().await.unwrap();
    let response = test_case.login(john_credentials()).await;
    let response_body = response.json::<RawLoginResponseBody>().await.unwrap();
    (user, response_body)
}

/// Check that the cookie specification for access/refresh tokens is correct
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
