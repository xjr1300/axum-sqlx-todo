use axum::{Router, middleware, routing::get};

use infra::{
    AppState,
    http::{
        handler::todo::{by_id, list},
        middleware::authorized_user_middleware,
    },
};

pub fn create_todo_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{todo_id}", get(by_id))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state)
}
