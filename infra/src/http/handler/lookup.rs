use axum::{Json, extract::State};

use crate::{
    AppState,
    http::{ApiError, ApiResult},
};

pub mod role {
    use super::*;

    use domain::models::{Role, RoleCode};
    use use_case::lookup::{LookupUseCase, RoleUseCase};

    use crate::postgres::repositories::PgRoleRepository;

    #[tracing::instrument(skip(app_state))]
    pub async fn list(State(app_state): State<AppState>) -> ApiResult<Json<Vec<Role>>> {
        let pool = app_state.pg_pool.clone();
        let repo = PgRoleRepository { pool };
        let use_case = RoleUseCase { repo };
        Ok(Json(use_case.list().await.map_err(ApiError::from)?))
    }

    #[tracing::instrument(skip(app_state))]
    pub async fn by_code(
        State(app_state): State<AppState>,
        code: axum::extract::Path<i16>,
    ) -> ApiResult<Json<Option<Role>>> {
        let code = RoleCode::try_from(code.0).map_err(ApiError::from)?;
        let pool = app_state.pg_pool.clone();
        let repo = PgRoleRepository { pool };
        let use_case = RoleUseCase { repo };
        let role = use_case.by_code(&code).await.map_err(ApiError::from)?;
        Ok(Json(role))
    }
}

pub mod todo_status {
    use super::*;

    use domain::models::{TodoStatus, TodoStatusCode};
    use use_case::lookup::{LookupUseCase, TodoStatusUseCase};

    use crate::postgres::repositories::PgTodoStatusRepository;

    #[tracing::instrument(skip(app_state))]
    pub async fn list(State(app_state): State<AppState>) -> ApiResult<Json<Vec<TodoStatus>>> {
        let pool = app_state.pg_pool.clone();
        let repo = PgTodoStatusRepository { pool };
        let use_case = TodoStatusUseCase { repo };
        Ok(Json(use_case.list().await.map_err(ApiError::from)?))
    }

    #[tracing::instrument(skip(app_state))]
    pub async fn by_code(
        State(app_state): State<AppState>,
        code: axum::extract::Path<i16>,
    ) -> ApiResult<Json<Option<TodoStatus>>> {
        let code = TodoStatusCode::try_from(code.0).map_err(ApiError::from)?;
        let pool = app_state.pg_pool.clone();
        let repo = PgTodoStatusRepository { pool };
        let use_case = TodoStatusUseCase { repo };
        let role = use_case.by_code(&code).await.map_err(ApiError::from)?;
        Ok(Json(role))
    }
}
