use std::time::Duration;

use anyhow::Context as _;
use config::Config;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

use infra::postgres::repositories::create_pg_repositories;

use app::settings::AppSettings;

/// アプリケーションエントリーポイント
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // アプリケーション設定を読み込み
    let config = Config::builder()
        .add_source(config::File::with_name("app_settings.toml"))
        .build()
        .context("Failed to read the app_settings.toml file")?;
    let app_settings: AppSettings = config
        .try_deserialize()
        .context("The contents of the app_settings.toml file is incorrect")?;
    println!("App settings: {:?}", app_settings);

    // データベース接続プールを作成
    let pool = PgPoolOptions::new()
        .max_connections(app_settings.database.max_connections)
        .acquire_timeout(Duration::from_secs(
            app_settings.database.connection_timeout,
        ))
        .connect(&app_settings.database.uri())
        .await
        .context("Failed to connect to the database")?;

    // リポジトリコレクションを作成
    let repositories = create_pg_repositories(pool);

    // ルーターを作成
    let router = app::routes::create_router(repositories);

    // HTTPサーバーを起動
    let address = app_settings.http_server.bind_address();
    let listener = TcpListener::bind(&address)
        .await
        .context("Failed to bind to the address for the HTTP server")?;
    println!("HTTP server is running on {}", address);
    axum::serve(listener, router)
        .await
        .context("Failed to start the HTTP server")?;

    Ok(())
}
