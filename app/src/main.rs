use std::time::Duration;

use anyhow::Context as _;
use config::Config;
use deadpool_redis::Config as RedisConfig;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

use infra::AppState;
use settings::{AppSettings, DatabaseSettings, RedisSettings};

use app::routes::create_router;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt as _};

/// アプリケーションエントリーポイント
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // アプリケーション設定を読み込み
    let app_settings = load_app_settings()?;

    let subscriber = get_subscriber(
        "axum-sqlx-todo".into(),
        app_settings.log_level,
        std::io::stdout,
    );
    init_subscriber(subscriber);
    tracing::info!("{:?}", app_settings);

    // HTTPサーバーがバインドするアドレスを構築
    let address = app_settings.http_server.bind_address();
    // データベースコネクションプールを作成
    let pg_pool = create_pg_pool(&app_settings.database).await?;
    // Redisコネクションプールを作成
    let redis_pool = create_redis_pool(&app_settings.redis).await?;

    // ルーターを作成
    let app_state = AppState {
        app_settings,
        pg_pool,
        redis_pool,
    };
    let router = create_router(app_state);

    // HTTPサーバーを起動
    let listener = TcpListener::bind(&address)
        .await
        .context("Failed to bind to the address for the HTTP server")?;
    tracing::info!("HTTP server is running on {}", address);
    axum::serve(listener, router)
        .await
        .context("Failed to start the HTTP server")?;

    Ok(())
}

fn load_app_settings() -> anyhow::Result<AppSettings> {
    let config = Config::builder()
        .add_source(config::File::with_name("app_settings.toml"))
        .build()
        .context("Failed to read the app_settings.toml file")?;
    config
        .try_deserialize()
        .context("The contents of the app_settings.toml file is incorrect")
}

async fn create_pg_pool(settings: &DatabaseSettings) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(Duration::from_secs(settings.connection_timeout))
        .connect(&settings.uri())
        .await
        .context("Failed to connect to the database")
}

async fn create_redis_pool(settings: &RedisSettings) -> anyhow::Result<deadpool_redis::Pool> {
    let config = RedisConfig {
        url: Some(settings.uri()),
        connection: None,
        pool: None,
    };
    config
        .create_pool(None)
        .context("Failed to create Redis connection pool")
}

fn get_subscriber<Sink>(
    name: String,
    log_level: log::Level,
    sink: Sink,
) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level.to_string()));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
