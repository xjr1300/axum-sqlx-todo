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
//! A test database is created for each test run, so you must run the `bin/drop_test_dbs.sh` script
//! to drop the all test databases.
use std::{path::Path, thread::JoinHandle, time::Duration};

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::{Connection as _, Executor as _, PgConnection, PgPool};
// use time::{OffsetDateTime, serde::rfc3339};
use tokio::{net::TcpListener, sync::oneshot};

use app::{bind_address, create_redis_pool, load_app_settings, routes::create_router};
use domain::{
    models::{User, UserId},
    repositories::{TokenContent, TokenRepository, UserRepository},
};
use infra::{
    AppState, postgres::repositories::PgUserRepository, redis::token::RedisTokenRepository,
};
use settings::{AppSettings, DatabaseSettings};

pub const TEST_DATABASE_PREFIX: &str = "test_todo_db_";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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
    log: bool,
    pub http_client: reqwest::Client,
}

impl TestCase {
    pub async fn begin(log: bool) -> Self {
        let app = configure_test_app().await;
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
            log,
            http_client,
        }
    }

    pub fn origin(&self) -> String {
        format!(
            "{}://{}:{}",
            self.app_state.app_settings.http.protocol,
            self.app_state.app_settings.http.host,
            self.app_state.app_settings.http.port,
        )
    }

    pub async fn user_by_id(&self, id: UserId) -> Option<User> {
        let user_repo = PgUserRepository::new(self.app_state.pg_pool.clone());
        user_repo.by_id(id).await.unwrap()
    }

    pub async fn token_content_by_token(&self, token: &SecretString) -> Option<TokenContent> {
        let token_repo = RedisTokenRepository::new(self.app_state.redis_pool.clone());
        token_repo.get_token_content(token).await.unwrap()
    }

    pub async fn end(self) {
        if self.log {
            println!("Sending graceful shutdown signal...");
        }
        self.shutdown_signal.send(()).unwrap();
        if self.log {
            println!("Waiting for server to gracefully shutdown...");
        }
        self.app_handle.join().unwrap();
        if self.log {
            println!("Server has gracefully shutdown.");
        }
    }

    pub async fn sign_up(&self, body: &RawSignUpRequestBody) -> reqwest::Response {
        let uri = format!("{}/users/sign-up", self.origin());
        self.http_client.post(&uri).json(body).send().await.unwrap()
    }

    pub async fn login(&self, body: &RawLoginRequestBody) -> reqwest::Response {
        let uri = format!("{}/users/login", self.origin());
        self.http_client.post(&uri).json(body).send().await.unwrap()
    }
}

async fn configure_test_app() -> TestApp {
    // Load the application settings
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let path = Path::new(&dir).join("..").join("app_settings.toml");
    let mut app_settings = load_app_settings(path.as_os_str().to_str().unwrap()).unwrap();

    // Set up the test database
    let database_name =
        format!("{}{}", TEST_DATABASE_PREFIX, uuid::Uuid::new_v4()).replace('-', "_");
    app_settings.database.name = database_name; // テスト用のデータベース名を設定
    let pg_pool = setup_database(&app_settings.database).await;

    // Set up the Redis connection pool
    let redis_pool = create_redis_pool(&app_settings.redis).await.unwrap();

    // Specify a random port for the HTTP server to bind
    app_settings.http.port = 0;
    let (listener, port) = bind_address(&app_settings.http).await.unwrap();
    app_settings.http.port = port;

    TestApp {
        app_settings,
        listener,
        pg_pool,
        redis_pool,
    }
}

pub struct TestApp {
    pub app_settings: AppSettings,
    pub listener: TcpListener,
    pub pg_pool: PgPool,
    pub redis_pool: deadpool_redis::Pool,
}

/// Sets up the PostgreSQL database for testing
async fn setup_database(settings: &DatabaseSettings) -> PgPool {
    // Connect to the **postgres** database
    let postgres_settings = DatabaseSettings {
        name: String::from("postgres"),
        ..settings.clone()
    };
    let mut conn = PgConnection::connect_with(&postgres_settings.connect_options())
        .await
        .unwrap();

    // Create the test database
    conn.execute(format!("CREATE DATABASE {};", settings.name).as_str())
        .await
        .unwrap();

    // Migrate the database
    let pool = PgPool::connect_with(settings.connect_options())
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();

    pool
}

/// Spawns the application server in a separate thread
///
/// Returns a tuple containing the thread handle and a sender to signal for graceful shutdown.
async fn spawn_app(
    app_state: AppState,
    listener: TcpListener,
) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (close_tx, close_rx) = oneshot::channel();

    let handle = std::thread::spawn(|| run_server(app_state, listener, close_rx));
    (handle, close_tx)
}

/// Runs the application server with graceful shutdown support
fn run_server(app_state: AppState, listener: TcpListener, close_rx: oneshot::Receiver<()>) {
    let router = create_router(app_state);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                _ = close_rx.await;
            })
            .await
            .unwrap();
    });
}

pub struct ResponseParts {
    /// ステータスコード
    pub status_code: reqwest::StatusCode,
    /// ヘッダ
    pub headers: reqwest::header::HeaderMap,
    /// ボディ
    pub body: String,
}

pub async fn split_response(response: reqwest::Response) -> ResponseParts {
    ResponseParts {
        status_code: response.status(),
        headers: response.headers().clone(),
        body: response.text().await.unwrap().to_string(),
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
    // #[serde(deserialize_with = "rfc3339::deserialize")]
    // pub access_expiration: OffsetDateTime,
    pub refresh_token: String,
    // #[serde(deserialize_with = "rfc3339::deserialize")]
    // pub refresh_expiration: OffsetDateTime,
}
