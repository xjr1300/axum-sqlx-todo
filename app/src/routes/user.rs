use axum::{
    Router, middleware,
    routing::{get, patch, post},
};

use infra::{
    AppState,
    http::{
        handler::user::{login, logout, me, sign_up, update},
        middleware::authorized_user_middleware,
    },
};

pub fn create_user_routes(app_state: AppState) -> Router<AppState> {
    let router = Router::new()
        .route("/sign-up", post(sign_up))
        .route("/login", post(login))
        .with_state(app_state.clone());
    let protected_router = Router::new()
        .route("/me", get(me))
        .route("/me", patch(update))
        .route("/logout", post(logout))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state);
    Router::new().merge(router).merge(protected_router)
}
