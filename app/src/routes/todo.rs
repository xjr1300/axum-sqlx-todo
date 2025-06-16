use axum::{
    Router, middleware,
    routing::{get, post},
};

use infra::{
    AppState,
    http::{
        handler::todo::{archive, by_id, complete, create, list, reopen, update},
        middleware::authorized_user_middleware,
    },
};

pub fn create_todo_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{todo_id}/complete", post(complete))
        .route("/{todo_id}/reopen", post(reopen))
        .route("/{todo_id}/archive", post(archive))
        .route("/{todo_id}", get(by_id).patch(update))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state)
}
