pub mod user;

use axum::{Router, middleware, routing::get};

use infra::{
    AppState,
    http::{handler::health_check, middleware::authorized_user_middleware},
};
use user::create_user_routes;

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
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state)
}
