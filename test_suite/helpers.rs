use std::{path::Path, thread::JoinHandle};

use sqlx::{Connection as _, Executor as _, PgConnection, PgPool};
use tokio::{net::TcpListener, sync::oneshot};

use app::{bind_address, create_redis_pool, routes::create_router};
use infra::{
    AppState,
    settings::{AppSettings, DatabaseSettings, load_app_settings},
};

pub const TEST_DATABASE_PREFIX: &str = "test_todo_db_";

// use once_cell::sync::Lazy;
// use uuid::Uuid;
// use domain::models::UserId;
// pub static TARO_USER_ID: Lazy<UserId> =
//     Lazy::new(|| UserId::from(Uuid::parse_str("47125c09-1dea-42b2-a14e-357e59acf3dc").unwrap()));

pub struct TestApp {
    pub app_settings: AppSettings,
    pub listener: TcpListener,
    pub pg_pool: PgPool,
    pub redis_pool: deadpool_redis::Pool,
}

pub fn load_app_settings_for_testing() -> AppSettings {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    let path = Path::new(&dir).join("..").join("app_settings.toml");
    let mut settings = load_app_settings(path.as_os_str().to_str().unwrap()).unwrap();
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        settings.log_level = log_level_from_str(&rust_log);
    }
    // let subscriber =
    //     app::get_subscriber("axum-sqlx-todo".into(), settings.log_level, std::io::stdout);
    // app::init_subscriber(subscriber);
    settings
}

fn log_level_from_str(s: &str) -> log::Level {
    match s.to_lowercase().as_str() {
        "error" => log::Level::Error,
        "warn" => log::Level::Warn,
        "info" => log::Level::Info,
        "debug" => log::Level::Debug,
        "trace" => log::Level::Trace,
        _ => log::Level::Info,
    }
}

pub async fn configure_test_app(mut app_settings: AppSettings) -> TestApp {
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

async fn connect_to_postgres_database(settings: &DatabaseSettings) -> PgConnection {
    let postgres_settings = DatabaseSettings {
        name: String::from("postgres"),
        ..settings.clone()
    };
    PgConnection::connect_with(&postgres_settings.connect_options())
        .await
        .expect("Failed to connect to PostgreSQL database")
}

/// Sets up the PostgreSQL database for testing
async fn setup_database(settings: &DatabaseSettings) -> PgPool {
    // Connect to the **postgres** database
    let mut conn = connect_to_postgres_database(settings).await;

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
pub async fn spawn_app(
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
