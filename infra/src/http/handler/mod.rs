pub mod user;

/// ヘルスチェックハンドラ
#[tracing::instrument()]
pub async fn health_check() -> &'static str {
    "Ok, the server is running!"
}
