pub mod routes;

use std::time::Duration;

use anyhow::Context as _;
use config::Config;
use deadpool_redis::Config as RedisConfig;
use sqlx::postgres::PgPoolOptions;

use tokio::net::TcpListener;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt as _};

use infra::settings::{AppSettings, DatabaseSettings, HttpSettings, RedisSettings};

pub fn load_app_settings(path: &str) -> anyhow::Result<AppSettings> {
    let config = Config::builder()
        .add_source(config::File::with_name(path))
        .build()
        .context("Failed to read the app_settings.toml file")?;
    config
        .try_deserialize()
        .context("The contents of the app_settings.toml file is incorrect")
}

pub async fn bind_address(settings: &HttpSettings) -> anyhow::Result<(TcpListener, u16)> {
    let listener = TcpListener::bind(settings.bind_address())
        .await
        .context("Failed to bind to the address for the HTTP server")?;
    let port = listener
        .local_addr()
        .context("Failed to get the port of listener")?
        .port();

    Ok((listener, port))
}

pub async fn create_pg_pool(
    settings: &DatabaseSettings,
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(Duration::from_secs(settings.connection_timeout))
        .connect_with(settings.connect_options())
        .await
        .context("Failed to connect to the database")
}

pub async fn create_redis_pool(settings: &RedisSettings) -> anyhow::Result<deadpool_redis::Pool> {
    let config = RedisConfig {
        url: Some(settings.uri()),
        connection: None,
        pool: None,
    };
    config
        .create_pool(None)
        .context("Failed to create Redis connection pool")
}

pub fn get_subscriber<Sink>(
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

pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
