use time::macros::{date, datetime};
use uuid::Uuid;

use domain::models::Todo;
use infra::http::handler::todo::TodoListQueryParams;

use crate::{
    helpers::{ResponseParts, load_app_settings_for_testing, split_response},
    test_case::{EnableTracing, InsertTestData, REQUEST_TIMEOUT, TestCase},
};

/// Check that the user can get their own todo list.
#[tokio::test]
#[ignore]
async fn the_user_can_get_their_own_todo_list() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let response = test_case.todo_list(None).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, reqwest::StatusCode::OK, "{}", body);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    assert_eq!(todos.len(), 6);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    let todo = todos
        .iter()
        .find(|&t| t.id == Uuid::parse_str("ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175").unwrap())
        .unwrap();
    assert_eq!(
        todo.user.id,
        Uuid::parse_str("47125c09-1dea-42b2-a14e-357e59acf3dc").unwrap()
    );
    assert_eq!(todo.title, "レポート提出");
    assert_eq!(
        todo.description.as_ref().unwrap(),
        &"月次レポートを作成して提出"
    );
    assert_eq!(todo.status.code, 1);
    assert_eq!(todo.due_date, Some(date!(2025 - 06 - 12)));
    assert_eq!(todo.completed_at, None);
    assert!(!todo.archived);
    assert_eq!(todo.created_at, datetime!(2025-06-08 06:30:00 +09:00));
    assert_eq!(todo.updated_at, datetime!(2025-06-08 07:00:00 +09:00));

    let body = TodoListQueryParams::default();
    let response = test_case.todo_list(Some(body)).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, reqwest::StatusCode::OK, "{}", body);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    assert_eq!(todos.len(), 6);

    test_case.end().await;
}

/// Check that the user can get their own todo list by specifying the keyword.
#[tokio::test]
#[ignore]
async fn the_user_can_get_their_own_todo_list_by_keyword() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let body = TodoListQueryParams {
        keyword: Some(String::from("書籍")),
        ..Default::default()
    };
    let response = test_case.todo_list(Some(body)).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, reqwest::StatusCode::OK, "{}", body);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    assert_eq!(todos.len(), 2);

    test_case.end().await;
}

/// Check that the user can get their own todo list by specifying due date.
#[tokio::test]
#[ignore]
async fn the_user_can_get_their_own_todo_list_by_due_date() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let cases = [
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Eq),
                from: Some(date!(2025 - 06 - 12)),
                to: None,
                ..Default::default()
            },
            3,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Ne),
                from: Some(date!(2025 - 06 - 12)),
                to: None,
                ..Default::default()
            },
            2,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Gt),
                from: Some(date!(2025 - 06 - 15)),
                to: None,
                ..Default::default()
            },
            1,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Gte),
                from: Some(date!(2025 - 06 - 15)),
                to: None,
                ..Default::default()
            },
            2,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Lt),
                from: Some(date!(2025 - 06 - 18)),
                to: None,
                ..Default::default()
            },
            4,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Lte),
                from: Some(date!(2025 - 06 - 18)),
                to: None,
                ..Default::default()
            },
            5,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Between),
                from: Some(date!(2025 - 06 - 15)),
                to: Some(date!(2025 - 06 - 18)),
                ..Default::default()
            },
            2,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::NotBetween),
                from: Some(date!(2025 - 06 - 14)),
                to: Some(date!(2025 - 06 - 18)),
                ..Default::default()
            },
            3,
        ),
    ];

    test_case.login_taro().await;
    for (body, expected) in cases {
        let response = test_case.todo_list(Some(body)).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, reqwest::StatusCode::OK, "{}", body);
        let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
        assert_eq!(todos.len(), expected, "{}", body);
    }

    test_case.end().await;
}

/// Check that the user can get their own todo list by specifying todo statuses
#[tokio::test]
#[ignore]
async fn the_user_can_get_their_own_todo_list_by_todo_statuses() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let cases = [
        (
            TodoListQueryParams {
                statuses: Some(vec![1]),
                ..Default::default()
            },
            2,
        ),
        (
            TodoListQueryParams {
                statuses: Some(vec![1, 3, 4]),
                ..Default::default()
            },
            4,
        ),
    ];

    test_case.login_taro().await;
    for (body, expected) in cases {
        let response = test_case.todo_list(Some(body)).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, reqwest::StatusCode::OK, "{}", body);
        let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
        assert_eq!(todos.len(), expected, "{}", body);
    }

    test_case.end().await;
}

/// Check that the anonymous user can not access the todo list endpoint.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_access_the_todo_list_endpoint() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let uri = format!("{}/todos", test_case.origin());
    let response = client.get(&uri).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn get_todo_by_id_integration_test() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let valid_todo_id = "ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175";
    // If the user specifies the ID of a todo that belongs to them, they can get the todo.
    let response = test_case.todo_get_by_id(valid_todo_id).await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "{}",
        response.text().await.unwrap()
    );

    // If the ID of a todo is not invalid format, the user gets an error.
    let response = test_case.todo_get_by_id("invalid-todo-id").await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "{}",
        response.text().await.unwrap()
    );

    // If the todo with the user's specified ID belongs to another user, the user gets an error.
    let response = test_case
        .todo_get_by_id("653acf81-a2e6-43cb-b4b4-9cdb822c740e")
        .await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::FORBIDDEN,
        "{}",
        response.text().await.unwrap()
    );

    // If the user specifies the ID of a todo that does not exist, they get an error.
    let todo_id = Uuid::new_v4().to_string();
    let response = test_case.todo_get_by_id(&todo_id).await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "{}",
        response.text().await.unwrap()
    );

    // If an anonymous user tries to get a todo, they get an error.
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let uri = format!("{}/todos/{}", test_case.origin(), valid_todo_id);
    let response = client.get(&uri).send().await.unwrap();
    assert_eq!(
        response.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "{}",
        response.text().await.unwrap()
    );

    test_case.end().await;
}
