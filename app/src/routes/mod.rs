pub mod lookup;
pub mod todo;
pub mod user;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};

use infra::{AppState, http::handler::health_check};
use tower_http::cors::CorsLayer;
use user::create_user_routes;

use crate::routes::{
    lookup::{create_role_routes, create_todo_status_routes},
    todo::create_todo_routes,
};

/// ルーターを作成する。
///
/// # 引数
///
/// * `app_settings`: アプリケーション設定
/// * `pool`: PostgreSQLコネクションプール
pub fn create_router(app_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([
            header::ORIGIN,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
            header::COOKIE,
        ])
        .allow_credentials(true);

    axum::Router::new()
        .route("/health-check", get(health_check))
        .nest("/users", create_user_routes(app_state.clone()))
        .nest("/todos", create_todo_routes(app_state.clone()))
        .nest("/roles", create_role_routes(app_state.clone()))
        .nest(
            "/todo-statuses",
            create_todo_status_routes(app_state.clone()),
        )
        .layer(cors)
        .with_state(app_state)
}
