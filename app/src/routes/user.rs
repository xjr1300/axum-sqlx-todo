use axum::{Router, routing::post};

use infra::{AppState, http::handler::user::sign_up};

pub fn create_user_routes(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users/sign-up", post(sign_up))
        .with_state(app_state)
}
