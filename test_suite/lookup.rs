use reqwest::StatusCode;

use domain::models::{Role, RoleCode, TodoStatus, TodoStatusCode};

use crate::helpers::{ResponseParts, load_app_settings_for_testing, split_response};
use crate::test_case::{EnableTracing, InsertTestData, TestCase};

#[tokio::test]
#[ignore]
async fn user_can_list_roles() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let response = test_case.role_list().await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(
        status_code,
        StatusCode::OK,
        "Role list request failed: {}",
        status_code
    );
    let roles = serde_json::from_str::<Vec<Role>>(&body).unwrap();
    assert_eq!(roles.len(), 2, "Expected 2 roles, found {}", roles.len());
    assert!(
        roles.iter().any(|r| r.code == RoleCode::Admin),
        "Admin role not found in the list"
    );
    assert!(
        roles.iter().any(|r| r.code == RoleCode::User),
        "Admin role not found in the list"
    );

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_list_roles() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    let response = test_case.role_list().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn user_can_get_a_role_by_code() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let expected = RoleCode::Admin;
    let response = test_case.role_by_code(expected as i16).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK,);
    let role = serde_json::from_str::<Role>(&body).unwrap();
    assert_eq!(role.code, expected);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_get_a_role_by_code() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    let expected = RoleCode::Admin;
    let response = test_case.role_by_code(expected as i16).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn user_can_list_todo_status_list() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let response = test_case.todo_status_list().await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(
        status_code,
        StatusCode::OK,
        "todo_status list request failed: {}",
        status_code
    );
    let todo_statuses = serde_json::from_str::<Vec<TodoStatus>>(&body).unwrap();
    let expected = [
        TodoStatusCode::NotStarted,
        TodoStatusCode::InProgress,
        TodoStatusCode::Completed,
        TodoStatusCode::Cancelled,
        TodoStatusCode::OnHold,
    ];
    assert_eq!(
        todo_statuses.len(),
        expected.len(),
        "Expected {} todo_statuses, found {}",
        expected.len(),
        todo_statuses.len()
    );
    for code in expected.iter() {
        assert!(
            todo_statuses.iter().any(|t| t.code == *code),
            "{} is not found in the list",
            code
        )
    }

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_list_todo_status_list() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    let response = test_case.todo_status_list().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn user_can_get_a_todo_status_by_code() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let expected = TodoStatusCode::InProgress;
    let response = test_case.todo_status_by_code(expected as i16).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK,);
    let todo_status = serde_json::from_str::<TodoStatus>(&body).unwrap();
    assert_eq!(todo_status.code, expected,);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_get_a_todo_status_by_code() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    let expected = TodoStatusCode::InProgress;
    let response = test_case.todo_status_by_code(expected as i16).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}
