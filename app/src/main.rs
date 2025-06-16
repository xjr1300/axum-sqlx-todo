use anyhow::Context as _;

use infra::{AppState, settings::load_app_settings};

use app::{
    bind_address, create_pg_pool, create_redis_pool, get_subscriber, init_subscriber,
    routes::create_router,
};

/// アプリケーションエントリーポイント
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // アプリケーション設定を読み込み
    let mut app_settings = load_app_settings("app_settings.toml")?;

    let subscriber = get_subscriber("rusty-todo".into(), app_settings.log_level, std::io::stdout);
    init_subscriber(subscriber);
    tracing::info!("{:?}", app_settings);

    // HTTPサーバーがバインドするアドレスを構築
    let (listener, port) = bind_address(&app_settings.http).await?;
    app_settings.http.port = port; // 実際にバインドしたポートを設定
    let address = app_settings.http.bind_address();

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
    tracing::info!("HTTP server is running on {}", address);
    axum::serve(listener, router)
        .await
        .context("Failed to start the HTTP server")?;

    Ok(())
}
