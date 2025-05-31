use infra::postgres::repositories::PgRepositories;

/// ルーターを作成する。
///
/// # 引数
///
/// * `repositories`: リポジトリコレクション
pub fn create_router(repositories: PgRepositories) -> axum::Router {
    axum::Router::new()
        .route("/health-check", axum::routing::get(health_check))
        .with_state(repositories)
}

/// ヘルスチェックハンドラ
pub async fn health_check() -> &'static str {
    "Ok, the server is running!"
}
