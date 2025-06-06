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

use sqlx::{Connection as _, Executor as _, PgConnection, PgPool};
use tokio::{net::TcpListener, sync::oneshot};

use app::{bind_address, create_redis_pool, load_app_settings, routes::create_router};
use infra::AppState;
use settings::{AppSettings, DatabaseSettings};

pub const TEST_DATABASE_PREFIX: &str = "test_todo_db_";

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
    pub client: reqwest::Client,
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
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .cookie_store(true)
            .build()
            .unwrap();
        Self {
            app_state,
            app_handle,
            shutdown_signal,
            log,
            client,
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
}

async fn configure_test_app() -> TestApp {
    // Load the application settings
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let path = Path::new(&dir).join("..").join("app_settings.toml");
    let mut app_settings = load_app_settings(path.as_os_str().to_str().unwrap()).unwrap();

    // Set up the test database
    let database_name =
        format!("{}_{}", TEST_DATABASE_PREFIX, uuid::Uuid::new_v4()).replace('-', "_");
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
