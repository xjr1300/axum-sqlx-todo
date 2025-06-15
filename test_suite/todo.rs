use axum::http::HeaderMap;
use reqwest::StatusCode;
use time::{
    OffsetDateTime,
    macros::{date, datetime},
};
use uuid::Uuid;

use domain::models::{Todo, TodoStatusCode};
use infra::http::handler::todo::TodoListQueryParams;

use crate::{
    helpers::{ResponseParts, load_app_settings_for_testing, split_response},
    test_case::{EnableTracing, InsertTestData, REQUEST_TIMEOUT, TARO_USER_ID, TestCase},
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
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    assert_eq!(todos.len(), 13);
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
    assert_eq!(todo.status.code, TodoStatusCode::NotStarted);
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
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
    assert_eq!(todos.len(), 13);

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
    assert_eq!(status_code, StatusCode::OK, "{}", body);
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
            2,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Ne),
                from: Some(date!(2025 - 06 - 12)),
                to: None,
                ..Default::default()
            },
            11,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Gt),
                from: Some(date!(2025 - 06 - 15)),
                to: None,
                ..Default::default()
            },
            5,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Gte),
                from: Some(date!(2025 - 06 - 15)),
                to: None,
                ..Default::default()
            },
            7,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Lt),
                from: Some(date!(2025 - 06 - 18)),
                to: None,
                ..Default::default()
            },
            7,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Lte),
                from: Some(date!(2025 - 06 - 18)),
                to: None,
                ..Default::default()
            },
            8,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::Between),
                from: Some(date!(2025 - 06 - 15)),
                to: Some(date!(2025 - 06 - 18)),
                ..Default::default()
            },
            5,
        ),
        (
            TodoListQueryParams {
                op: Some(domain::NumericOperator::NotBetween),
                from: Some(date!(2025 - 06 - 14)),
                to: Some(date!(2025 - 06 - 18)),
                ..Default::default()
            },
            7,
        ),
    ];

    test_case.login_taro().await;
    for (param, expected) in cases {
        let response = test_case.todo_list(Some(param.clone())).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::OK, "{}", body);
        let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
        assert_eq!(todos.len(), expected, "{}", param);
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
            6,
        ),
        (
            TodoListQueryParams {
                statuses: Some(vec![1, 3, 4]),
                ..Default::default()
            },
            10,
        ),
    ];

    test_case.login_taro().await;
    for (body, expected) in cases {
        let response = test_case.todo_list(Some(body)).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::OK, "{}", body);
        let todos = serde_json::from_str::<Vec<Todo>>(&body).unwrap();
        assert_eq!(todos.len(), expected, "{}", body);
    }

    test_case.end().await;
}

#[tokio::test]
#[ignore]
async fn the_user_can_get_their_own_todo_list_by_archived() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let cases = [
        (
            TodoListQueryParams {
                archived: Some(false),
                ..Default::default()
            },
            12,
        ),
        (
            TodoListQueryParams {
                archived: Some(true),
                ..Default::default()
            },
            1,
        ),
    ];

    test_case.login_taro().await;
    for (body, expected) in cases {
        let response = test_case.todo_list(Some(body)).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::OK, "{}", body);
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
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

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
        StatusCode::OK,
        "{}",
        response.text().await.unwrap()
    );

    // If the ID of a todo is not invalid format, the user gets an error.
    let response = test_case.todo_get_by_id("invalid-todo-id").await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "{}",
        response.text().await.unwrap()
    );

    // If the todo with the user's specified ID belongs to another user, the user gets an error.
    let response = test_case
        .todo_get_by_id("653acf81-a2e6-43cb-b4b4-9cdb822c740e")
        .await;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "{}",
        response.text().await.unwrap()
    );

    // If the user specifies the ID of a todo that does not exist, they get an error.
    let todo_id = Uuid::new_v4().to_string();
    let response = test_case.todo_get_by_id(&todo_id).await;
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
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
        StatusCode::UNAUTHORIZED,
        "{}",
        response.text().await.unwrap()
    );

    test_case.end().await;
}

/// Check that the user can create a todo with a due date.
#[tokio::test]
#[ignore]
async fn create_todo_with_due_date() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let request_body = String::from(
        r#"
        {
            "title": "Rustの学習",
            "description": "Rustの非同期処理を学ぶ",
            "dueDate": "2025-06-20"
        }
        "#,
    );
    let response = test_case.todo_create(request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::CREATED, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();
    assert_eq!(todo.user.id, *TARO_USER_ID);
    assert_eq!(todo.title, "Rustの学習");
    assert_eq!(
        todo.description.as_ref().unwrap(),
        &"Rustの非同期処理を学ぶ"
    );
    assert_eq!(todo.status.code, TodoStatusCode::NotStarted);
    assert_eq!(todo.due_date, Some(date!(2025 - 06 - 20)));
    assert_eq!(todo.completed_at, None);

    test_case.end().await;
}

// Check that the user can create a todo without a due date.
#[tokio::test]
#[ignore]
async fn create_todo_without_description_and_due_date() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let request_bodies = vec![
        String::from(
            r#"
            {
                "title": "Rustの学習",
                "description": null,
                "dueDate": null
            }
            "#,
        ),
        String::from(
            r#"
            {
                "title": "Rustの学習"
            }
            "#,
        ),
    ];
    for request_body in request_bodies {
        let response = test_case.todo_create(request_body.clone()).await;
        let ResponseParts { body, .. } = split_response(response).await;
        let todo = serde_json::from_str::<Todo>(&body).unwrap();
        assert!(todo.description.is_none());
        assert!(todo.due_date.is_none());
    }

    test_case.end().await;
}

/// Check that the anonymous user can not access the endpoint to create a todo.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_access_the_create_todo_endpoint() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let request_body = String::from(
        r#"
            {
                "title": "Rustの学習"
            }
            "#,
    );
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    let uri = format!("{}/todos", test_case.origin());
    let response = client
        .post(&uri)
        .headers(headers)
        .body(request_body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

/// Check that the user can update a todo.
#[tokio::test]
#[ignore]
async fn user_can_update_todo() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";
    let request_body = format!(
        r#"
        {{
            "title": "Rustの学習を深める",
            "description": "Rustの非同期処理とエラーハンドリングを学ぶ",
            "statusCode": {},
            "dueDate": "2025-06-30"
        }}
        "#,
        TodoStatusCode::NotStarted as i16
    );
    let requested_at = OffsetDateTime::now_utc();
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();

    assert_eq!(todo.title, "Rustの学習を深める");
    assert_eq!(
        todo.description.unwrap(),
        "Rustの非同期処理とエラーハンドリングを学ぶ"
    );
    assert_eq!(todo.status.code, TodoStatusCode::NotStarted);
    assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 30));
    assert!(todo.updated_at > requested_at);

    test_case.end().await;
}

/// Check that the user can update a todo with each specified field.
#[tokio::test]
#[ignore]
async fn user_can_update_todo_with_each_specified_field() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";

    // Update only the title of the todo
    let request_body = String::from(
        r#"
        {
            "title": "Rustの学習を深める"
        }
        "#,
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();
    assert_eq!(todo.title, "Rustの学習を深める");
    assert_eq!(todo.description.unwrap(), "プロジェクトの進捗確認");
    assert_eq!(todo.status.code, TodoStatusCode::InProgress);
    assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 12));

    // Update only the description of the todo
    let request_body = String::from(
        r#"
        {
            "description": "Rustの非同期処理とエラーハンドリングを学ぶ"
        }
        "#,
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();
    assert_eq!(todo.title, "Rustの学習を深める");
    assert_eq!(
        todo.description.unwrap(),
        "Rustの非同期処理とエラーハンドリングを学ぶ"
    );
    assert_eq!(todo.status.code, TodoStatusCode::InProgress);
    assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 12));

    // Update only the status of the todo
    let request_body = format!(
        r#"
        {{
            "statusCode": {}
        }}
        "#,
        TodoStatusCode::NotStarted as i16
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();
    assert_eq!(todo.title, "Rustの学習を深める");
    assert_eq!(
        todo.description.unwrap(),
        "Rustの非同期処理とエラーハンドリングを学ぶ"
    );
    assert_eq!(todo.status.code, TodoStatusCode::NotStarted);
    assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 12));

    // Update only the due date of the todo
    let request_body = String::from(
        r#"
        {
            "dueDate": "2025-06-30"
        }
        "#,
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::OK, "{}", body);
    let todo = serde_json::from_str::<Todo>(&body).unwrap();
    assert_eq!(todo.title, "Rustの学習を深める");
    assert_eq!(
        todo.description.unwrap(),
        "Rustの非同期処理とエラーハンドリングを学ぶ"
    );
    assert_eq!(todo.status.code, TodoStatusCode::NotStarted);
    assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 30));

    test_case.end().await;
}

/// Check that the todo is not changed if the user does not specify any fields to update.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_without_specifying_any_fields() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    let request_bodies = vec![
        String::from(
            r#"
            {
                "title": null,
                "description": null,
                "statusCode": null,
                "dueDate": null
            }
            "#,
        ),
        String::from("{}"),
    ];
    test_case.login_taro().await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";
    for request_body in request_bodies {
        let requested_at = OffsetDateTime::now_utc();
        let response = test_case.todo_update(todo_id, request_body.clone()).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::OK, "{}", body);
        let todo = serde_json::from_str::<Todo>(&body).unwrap();
        assert_eq!(todo.title, "チームミーティング");
        assert_eq!(todo.description.unwrap(), "プロジェクトの進捗確認");
        assert_eq!(todo.status.code, TodoStatusCode::InProgress);
        assert_eq!(todo.due_date.unwrap(), date!(2025 - 06 - 12));
        assert!(todo.updated_at > requested_at);
    }

    test_case.end().await;
}

/// Check that the user can not update a completed or archived todo.
#[tokio::test]
#[ignore]
async fn user_can_not_update_if_todo_is_completed_or_archived() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let completed_todo_id = "a0c1b2d3-4e5f-6789-abcd-ef0123456789";
    let archived_todo_id = "94904cc3-fff5-44c5-a290-0a6cd54902cd";

    test_case.login_taro().await;
    for todo_id in [completed_todo_id, archived_todo_id] {
        let request_body = String::from(
            r#"
            {
                "title": "更新できないタイトル",
                "description": "更新できない説明",
                "statusCode": 1,
                "dueDate": "2025-06-30"
            }
            "#,
        );
        let response = test_case.todo_update(todo_id, request_body).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST, "{}", body);
        assert!(
            body.contains("Cannot update completed or archived todo"),
            "{}",
            body
        );
    }

    test_case.end().await;
}

/// Check that the user can not update a todo that belongs to another user.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_that_belongs_to_another_user() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let another_user_todo_id = "653acf81-a2e6-43cb-b4b4-9cdb822c740e";
    let request_body = String::from(
        r#"
        {
            "title": "更新できないタイトル",
            "description": "更新できない説明",
            "statusCode": 1,
            "dueDate": "2025-06-30"
        }
        "#,
    );
    let response = test_case
        .todo_update(another_user_todo_id, request_body)
        .await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::FORBIDDEN, "{}", body);
    assert!(
        body.contains("You are not authorized to update this todo"),
        "{}",
        body
    );

    test_case.end().await;
}

/// Check that the user can not update a todo with a todo ID that is not recorded in any todos.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_that_is_not_recorded_in_any_todos() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let todo_id = Uuid::new_v4().to_string();
    let request_body = String::from(
        r#"
        {
            "title": "更新できないタイトル",
            "description": "更新できない説明",
            "statusCode": 1,
            "dueDate": "2025-06-30"
        }
        "#,
    );
    let response = test_case.todo_update(&todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::NOT_FOUND, "{}", body);
    assert!(body.contains("Todo not found"), "{}", body);

    test_case.end().await;
}

/// Check that the user can not update a todo with an invalid todo ID.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_if_user_specifies_invalid_todo_id() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let request_body = String::from(
        r#"
        {
            "id": "invalid-todo-id",
            "title": "更新できないタイトル",
            "description": "更新できない説明",
            "statusCode": -32768,
            "dueDate": "2025-06-30"
        }
        "#,
    );
    let response = test_case.todo_update("invalid-todo-id", request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST, "{}", body);
    assert!(body.contains("UUID parsing failed"), "{}", body);

    test_case.end().await;
}

/// Check that the user can not update a todo with an invalid status code.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_if_user_specifies_an_invalid_status_code() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";
    let request_body = String::from(
        r#"
        {
            "title": "更新できないタイトル",
            "description": "更新できない説明",
            "statusCode": -32768,
            "dueDate": "2025-06-30"
        }
        "#,
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST, "{}", body);
    assert!(body.contains("Invalid todo status code"), "{}", body);

    test_case.end().await;
}

/// Check that the user can not update a todo with an invalid due date.
#[tokio::test]
#[ignore]
async fn user_can_not_update_todo_if_user_specifies_an_invalid_due_date() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";
    let request_body = String::from(
        r#"
        {
            "dueDate": "2025-06-31"
        }
        "#,
    );
    let response = test_case.todo_update(todo_id, request_body).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::UNPROCESSABLE_ENTITY, "{}", body);
    assert!(body.contains("dueDate"), "{}", body);

    test_case.end().await;
}

/// Check that the anonymous user can not access the endpoint to update a todo.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_access_the_update_todo_endpoint() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;
    let todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let request_body = format!(
        r#"
        {{
            "title": "Rustの学習を深める",
            "description": "Rustの非同期処理とエラーハンドリングを学ぶ",
            "statusCode": {},
            "dueDate": "2025-06-30"
        }}
        "#,
        TodoStatusCode::NotStarted as i16
    );
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    let uri = format!("{}/todos/{}", test_case.origin(), todo_id);
    let response = client
        .patch(&uri)
        .headers(headers)
        .body(request_body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}

/// Check that the user can complete a todo.
#[tokio::test]
#[ignore]
async fn user_can_complete_todo() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let not_started_todo_id = "ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175";
    let in_progress_todo_id = "4da95cdb-6898-4739-b2be-62ceaa174baf";
    let completable_todo_ids = [not_started_todo_id, in_progress_todo_id];

    test_case.login_taro().await;
    for todo_id in completable_todo_ids {
        let requested_at = OffsetDateTime::now_utc();
        let response = test_case.todo_complete(todo_id).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::OK, "{}", body);
        let todo = serde_json::from_str::<Todo>(&body).unwrap();
        assert_eq!(todo.status.code, TodoStatusCode::Completed);
        assert!((todo.completed_at.unwrap() - requested_at).abs() < REQUEST_TIMEOUT);
    }

    test_case.end().await;
}

/// Check that the user can not complete a completed or archived todo.
#[tokio::test]
#[ignore]
async fn user_can_not_complete_a_completed_or_archived_todo() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;
    let completed_todo_id = "a61301fa-bb2a-490b-84aa-7dae6c4e086a";
    let cancelled_todo_id = "b1c2d3e4-5f6a-7890-abcd-ef0123456789";
    let on_hold_todo_id = "a61301fa-bb2a-490b-84aa-7dae6c4e086a";
    let archived_todo_id = "94904cc3-fff5-44c5-a290-0a6cd54902cd";
    let non_completable_todo_ids = [
        completed_todo_id,
        cancelled_todo_id,
        on_hold_todo_id,
        archived_todo_id,
    ];

    test_case.login_taro().await;
    for todo_id in non_completable_todo_ids {
        let response = test_case.todo_complete(todo_id).await;
        let ResponseParts {
            status_code, body, ..
        } = split_response(response).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST, "{}", body);
        assert!(
            body.contains("Only todos with status 'NotStarted' or 'InProgress'"),
            "{}",
            body
        );
    }

    test_case.end().await;
}

/// Check that the user can not complete a todo that belongs to another user.
#[tokio::test]
#[ignore]
async fn user_can_not_complete_a_todo_that_belongs_to_another_user() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::Yes).await;

    test_case.login_taro().await;
    let another_user_todo_id = "653acf81-a2e6-43cb-b4b4-9cdb822c740e";
    let response = test_case.todo_complete(another_user_todo_id).await;
    let ResponseParts {
        status_code, body, ..
    } = split_response(response).await;
    assert_eq!(status_code, StatusCode::FORBIDDEN, "{}", body);
    assert!(
        body.contains("You are not authorized to update this todo"),
        "{}",
        body
    );

    test_case.end().await;
}

// Check that anonymous user can not access the endpoint to complete a todo.
#[tokio::test]
#[ignore]
async fn anonymous_user_can_not_access_the_complete_todo_endpoint() {
    let app_settings = load_app_settings_for_testing();
    let test_case = TestCase::begin(app_settings, EnableTracing::No, InsertTestData::No).await;
    let todo_id = "ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175";

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .build()
        .unwrap();
    let uri = format!("{}/todos/{}/complete", test_case.origin(), todo_id);
    let response = client.post(&uri).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    test_case.end().await;
}
