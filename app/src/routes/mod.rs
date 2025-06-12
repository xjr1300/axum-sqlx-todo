pub mod todo;
pub mod user;

use axum::{Router, routing::get};

use infra::{AppState, http::handler::health_check};
use user::create_user_routes;

use crate::routes::todo::create_todo_routes;

/// ルーターを作成する。
///
/// # 引数
///
/// * `app_settings`: アプリケーション設定
/// * `pool`: PostgreSQLコネクションプール
pub fn create_router(app_state: AppState) -> Router {
    axum::Router::new()
        .route("/health-check", get(health_check))
        .nest("/users", create_user_routes(app_state.clone()))
        .nest("/todos", create_todo_routes(app_state.clone()))
        .with_state(app_state)
}
