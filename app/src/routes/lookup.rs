use axum::{Router, middleware, routing::get};

use infra::{
    AppState,
    http::{
        handler::lookup::{role, todo_status},
        middleware::authorized_user_middleware,
    },
};

pub fn create_role_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(role::list))
        .route("/{code}", get(role::by_code))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state)
}

pub fn create_todo_status_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(todo_status::list))
        .route("/{code}", get(todo_status::by_code))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state)
}
