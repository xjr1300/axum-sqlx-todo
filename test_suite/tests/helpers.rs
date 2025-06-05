//! This module sets up and runs integration tests
//!
//! The integration test uses the same PostgreSQL container as the development environment.
//! But, it creates a separate test database for integration tests.
//! The test database is named in the format `test_todo_db_<uuid>`,
//! where `<uuid>` is the UUID with hyphens replaced by underscores.
//!
//! The integration test uses the same Redis container as the development environment,
//! because the access tokens and refresh tokens are highly random.
use std::{path::Path, thread::JoinHandle};

use sqlx::{Connection as _, Executor as _, PgConnection, PgPool};
use tokio::{net::TcpListener, sync::oneshot};

use app::{bind_address, create_redis_pool, load_app_settings, routes::create_router};
use infra::AppState;
use settings::{AppSettings, DatabaseSettings};

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
    app_handle: JoinHandle<()>,
    shutdown_signal: oneshot::Sender<()>,
    port: u16,
    log: bool,
}

impl TestCase {
    pub async fn begin(log: bool) -> Self {
        let test_app = configure_test_app().await;
        let port = test_app.app_settings.http.port;
        let (app_handle, shutdown_signal) = spawn_app(test_app).await;
        Self {
            app_handle,
            shutdown_signal,
            port,
            log,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
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
    let database_name = format!("test_todo_db_{}", uuid::Uuid::new_v4()).replace('-', "_");
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

struct TestApp {
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
async fn spawn_app(app: TestApp) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (close_tx, close_rx) = oneshot::channel();

    let handle = std::thread::spawn(|| run_server(app, close_rx));
    (handle, close_tx)
}

/// Runs the application server with graceful shutdown support
fn run_server(app: TestApp, close_rx: oneshot::Receiver<()>) {
    let app_state = AppState {
        app_settings: app.app_settings,
        pg_pool: app.pg_pool,
        redis_pool: app.redis_pool,
    };
    let router = create_router(app_state);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        axum::serve(app.listener, router)
            .with_graceful_shutdown(async move {
                _ = close_rx.await;
            })
            .await
            .unwrap();
    });
}
