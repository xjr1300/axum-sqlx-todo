use axum::{Router, middleware, routing::get};

use infra::{
    AppState,
    http::{
        handler::lookup::{role, todo_status},
        middleware::authorized_user_middleware,
    },
};

macro_rules! create_lookup_routes {
    ($name:ident, $module:ident) => {
        pub fn $name(app_status: infra::AppState) -> Router<AppState> {
            Router::new()
                .route("/", get($module::list))
                .route("/{code}", get($module::by_code))
                .layer(middleware::from_fn_with_state(
                    app_status.clone(),
                    authorized_user_middleware,
                ))
                .with_state(app_status)
        }
    };
}

create_lookup_routes!(create_role_routes, role);
create_lookup_routes!(create_todo_status_routes, todo_status);
