//! This module sets up and runs integration tests
//!
//! The integration test uses the same PostgreSQL container as the development environment.
//! But, it creates a separate test database for integration tests.
//! The test database is named in the format `test_todo_db_<uuid>`,
//! where `<uuid>` is the UUID with hyphens replaced by underscores.
//!
//! The integration test uses the same Redis container as the development environment,
//! because the access tokens and refresh tokens are highly random.
//!
//! [NOTICE]
//!
//! A test database is created for each test run.
//! So you must run the `bin/drop_test_dbs.sh` script to drop all the test databases.
use std::{thread::JoinHandle, time::Duration};

use axum::http::HeaderMap;
use once_cell::sync::Lazy;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use app::{get_subscriber, init_subscriber};
use domain::{
    models::{LoginFailedHistory, User, UserId},
    repositories::{
        TokenContent, TokenRepository, UserRepository, UserToken, generate_auth_token_info_key,
    },
};
use infra::{
    AppState,
    http::handler::{todo::TodoListQueryParams, user::UpdateUserRequestBody},
    postgres::repositories::PgUserRepository,
    redis::token::RedisTokenRepository,
    settings::AppSettings,
};

use crate::helpers::{TestApp, configure_test_app, spawn_app};

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub static TARO_USER_ID: Lazy<Uuid> =
    Lazy::new(|| Uuid::parse_str("47125c09-1dea-42b2-a14e-357e59acf3dc").unwrap());

/// Test case for integration tests
///
/// ```
/// #[tokio::test]
/// async fn integration_test_case_skeleton() {
///     // Initialize the test case
///     let test_case = TestCase::begin(true).await;
///     println!("Test application started on port: {}", test_case.port());
///
///     /************************************************************
///
///             Implement integration test logic here
///
///     *************************************************************/
///
///     // Next lines simulate a graceful shutdown, so real test logic should not be included next lines
///     println!("Waiting for 3 seconds before sending graceful shutdown signal...");
///     std::thread::sleep(std::time::Duration::from_secs(3));
///
///     // Terminate the test case gracefully
///     test_case.end().await;
/// }
/// ```
pub struct TestCase {
    pub app_state: AppState,
    app_handle: JoinHandle<()>,
    shutdown_signal: oneshot::Sender<()>,
    pub http_client: reqwest::Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnableTracing {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTestData {
    Yes,
    No,
}

impl TestCase {
    pub async fn begin(
        app_settings: AppSettings,
        tracing: EnableTracing,
        insertion: InsertTestData,
    ) -> Self {
        let app = configure_test_app(app_settings.clone()).await;
        if insertion == InsertTestData::Yes {
            // Insert test data into the database if required
            let content = std::fs::read_to_string("./fixtures/test.sql").unwrap();
            sqlx::raw_sql(&content).execute(&app.pg_pool).await.unwrap();
        }
        if tracing == EnableTracing::Yes {
            let subscriber = get_subscriber(
                "axum-sqlx-todo".into(),
                app_settings.log_level,
                std::io::stdout,
            );
            init_subscriber(subscriber);
        }
        let TestApp {
            app_settings,
            listener,
            pg_pool,
            redis_pool,
        } = app;
        let app_state = AppState {
            app_settings,
            pg_pool,
            redis_pool,
        };
        let (app_handle, shutdown_signal) = spawn_app(app_state.clone(), listener).await;
        let http_client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .cookie_store(true)
            .build()
            .unwrap();
        Self {
            app_state,
            app_handle,
            shutdown_signal,
            http_client,
        }
    }

    pub async fn end(self) {
        tracing::trace!("Sending graceful shutdown signal...");
        self.shutdown_signal.send(()).unwrap();
        tracing::trace!("Waiting for server to gracefully shutdown...");
        self.app_handle.join().unwrap();
        tracing::trace!("Server has gracefully shutdown.");
    }

    pub fn origin(&self) -> String {
        format!(
            "{}://{}:{}",
            self.app_state.app_settings.http.protocol,
            self.app_state.app_settings.http.host,
            self.app_state.app_settings.http.port,
        )
    }

    pub async fn user_by_id(&self, user_id: UserId) -> Option<User> {
        let user_repo = PgUserRepository::new(self.app_state.pg_pool.clone());
        user_repo.by_id(user_id).await.unwrap()
    }

    pub async fn get_login_failed_history(&self, user_id: UserId) -> Option<LoginFailedHistory> {
        let user_repo = PgUserRepository::new(self.app_state.pg_pool.clone());
        user_repo.get_login_failed_history(user_id).await.unwrap()
    }

    pub async fn user_tokens_from_user_repo(&self, user_id: UserId) -> Vec<UserToken> {
        let user_repo = PgUserRepository::new(self.app_state.pg_pool.clone());
        user_repo.user_tokens_by_id(user_id).await.unwrap()
    }

    pub async fn token_content_from_token_repo(
        &self,
        token: &SecretString,
    ) -> Option<TokenContent> {
        let token_repo = RedisTokenRepository::new(self.app_state.redis_pool.clone());
        let key = generate_auth_token_info_key(token);
        token_repo.get_token_content(&key).await.unwrap()
    }

    pub async fn set_user_active_status(&self, user_id: UserId, active: bool) {
        let mut tx = self.app_state.pg_pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE users SET active = $1 WHERE id = $2",
            active,
            user_id.0
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    pub async fn sign_up(&self, body: &RawSignUpRequestBody) -> reqwest::Response {
        let uri = format!("{}/users/sign-up", self.origin());
        self.http_client.post(&uri).json(body).send().await.unwrap()
    }

    pub async fn login(&self, body: &RawLoginRequestBody) -> reqwest::Response {
        let uri = format!("{}/users/login", self.origin());
        self.http_client.post(&uri).json(body).send().await.unwrap()
    }

    pub async fn me(&self) -> reqwest::Response {
        let uri = format!("{}/users/me", self.origin());
        self.http_client.get(&uri).send().await.unwrap()
    }

    pub async fn update_user(&self, body: &UpdateUserRequestBody) -> reqwest::Response {
        let uri = format!("{}/users/me", self.origin());
        self.http_client
            .patch(&uri)
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn refresh_tokens(&self) -> reqwest::Response {
        let uri = format!("{}/users/refresh-tokens", self.origin());
        self.http_client.post(&uri).send().await.unwrap()
    }

    pub async fn logout(&self) -> reqwest::Response {
        let uri = format!("{}/users/logout", self.origin());
        self.http_client.post(&uri).send().await.unwrap()
    }

    pub async fn login_taro(&self) {
        let body = RawLoginRequestBody {
            email: String::from("taro@example.com"),
            password: String::from("ab12AB#$"),
        };
        self.login(&body).await;
    }

    pub async fn todo_list(&self, params: Option<TodoListQueryParams>) -> reqwest::Response {
        let uri = format!("{}/todos", self.origin());
        match params {
            Some(body) => {
                let params = body.to_string();
                let uri = format!("{}?{}", uri, params);
                self.http_client.get(&uri).send().await.unwrap()
            }
            None => self.http_client.get(&uri).send().await.unwrap(),
        }
    }

    pub async fn todo_get_by_id(&self, tood_id: &str) -> reqwest::Response {
        let uri = format!("{}/todos/{}", self.origin(), tood_id);
        self.http_client.get(&uri).send().await.unwrap()
    }

    pub async fn todo_create(&self, body: String) -> reqwest::Response {
        let mut headers = HeaderMap::new();
        headers.append("Content-Type", "application/json".parse().unwrap());
        let uri = format!("{}/todos", self.origin());
        self.http_client
            .post(&uri)
            .headers(headers)
            .body(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn todo_update(&self, todo_id: &str, body: String) -> reqwest::Response {
        let mut headers = HeaderMap::new();
        headers.append("Content-Type", "application/json".parse().unwrap());
        let uri = format!("{}/todos/{}", self.origin(), todo_id);
        self.http_client
            .patch(&uri)
            .headers(headers)
            .body(body)
            .send()
            .await
            .unwrap()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSignUpRequestBody {
    pub family_name: String,
    pub given_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawLoginRequestBody {
    pub email: String,
    pub password: String,
}

impl From<RawSignUpRequestBody> for RawLoginRequestBody {
    fn from(value: RawSignUpRequestBody) -> Self {
        RawLoginRequestBody {
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawLoginResponseBody {
    pub access_token: String,
    pub refresh_token: String,
}
