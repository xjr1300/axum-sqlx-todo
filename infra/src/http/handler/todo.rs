use axum::extract::Query;
use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use time::Date;

use domain::{
    NumericOperator,
    models::{Todo, TodoStatusCode},
    repositories::TodoListInput,
};
use use_case::{AuthorizedUser, todo::TodoUseCase};
use utils::{serde::deserialize_option_split_comma, time::DATE_FORMAT};

use crate::{
    AppState,
    http::{ApiError, ApiResult},
    postgres::repositories::PgTodoRepository,
};

#[tracing::instrument(skip(app_state))]
pub async fn list(
    State(app_state): State<AppState>,
    Extension(user): Extension<AuthorizedUser>,
    query: Query<TodoListQueryParams>,
) -> ApiResult<Json<Vec<Todo>>> {
    let TodoListQueryParams {
        keyword,
        op,
        from,
        to,
        statuses,
    } = query.0;

    let statuses = if let Some(statuses) = statuses {
        Some(
            statuses
                .iter()
                .map(|s| TodoStatusCode::new(*s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ApiError::from)?,
        )
    } else {
        None
    };
    let input =
        TodoListInput::new(user.0.id, keyword, op, from, to, statuses).map_err(ApiError::from)?;
    let todo_repo = PgTodoRepository::new(app_state.pg_pool.clone());
    let use_case = TodoUseCase::new(todo_repo);
    let todos = use_case.list(input).await.map_err(ApiError::from)?;
    Ok(Json(todos))
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoListQueryParams {
    /// 検索キーワード
    pub keyword: Option<String>,
    /// 完了予定日検索の演算子
    pub op: Option<NumericOperator>,
    /// 完了予定日の開始日
    pub from: Option<Date>,
    /// 完了予定日の終了日
    pub to: Option<Date>,
    /// タスクのステータス
    #[serde(deserialize_with = "deserialize_option_split_comma")]
    pub statuses: Option<Vec<i16>>,
}

impl std::fmt::Display for TodoListQueryParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut params: Vec<String> = vec![];
        if let Some(keyword) = &self.keyword {
            params.push(format!("keyword={}", keyword));
        }
        if let Some(op) = self.op {
            params.push(format!("op={}", op));
        }
        if let Some(from) = self.from {
            params.push(format!("from={}", from.format(&DATE_FORMAT).unwrap()));
        }
        if let Some(to) = self.to {
            params.push(format!("to={}", to.format(&DATE_FORMAT).unwrap()));
        }
        if let Some(statuses) = &self.statuses {
            params.push(format!(
                "statuses={}",
                statuses
                    .iter()
                    .map(|status| status.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ));
        }
        write!(f, "{}", params.join("&"))
    }
}
