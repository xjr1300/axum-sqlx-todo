use axum::{Router, routing::post};

use infra::{
    AppState,
    http::handler::user::{login, sign_up},
};

pub fn create_user_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/sign-up", post(sign_up))
        .route("/login", post(login))
        .with_state(app_state)
}
