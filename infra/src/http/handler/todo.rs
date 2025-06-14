use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use domain::{
    NumericOperator,
    models::{Todo, TodoDescription, TodoId, TodoStatusCode, TodoTitle},
    repositories::{TodoCreateInput, TodoListInput, TodoUpdateInput},
};
use use_case::AuthorizedUser;
use utils::{
    serde::{deserialize_option_date, deserialize_option_split_comma, serialize_option_date},
    time::DATE_FORMAT,
};

use crate::{
    AppState,
    http::{ApiError, ApiResult, handler::todo_use_case, not_found},
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
                .map(|s| TodoStatusCode::try_from(*s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ApiError::from)?,
        )
    } else {
        None
    };
    let input =
        TodoListInput::new(user.0.id, keyword, op, from, to, statuses).map_err(ApiError::from)?;
    let use_case = todo_use_case(&app_state);
    let todos = use_case.list(input).await.map_err(ApiError::from)?;
    Ok(Json(todos))
}

pub async fn by_id(
    State(app_state): State<AppState>,
    Extension(auth_user): Extension<AuthorizedUser>,
    todo_id: Path<Uuid>,
) -> ApiResult<Json<Todo>> {
    let todo_id = TodoId::from(todo_id.0);
    let use_case = todo_use_case(&app_state);
    let todo = use_case
        .by_id(auth_user, todo_id)
        .await
        .map_err(ApiError::from)?;
    match todo {
        Some(todo) => Ok(Json(todo)),
        None => Err(not_found("todo")),
    }
}

pub async fn create(
    State(app_state): State<AppState>,
    Extension(auth_user): Extension<AuthorizedUser>,
    Json(body): Json<TodoCreateRequestBody>,
) -> ApiResult<impl IntoResponse> {
    let input = TodoCreateInput::try_from(body)?;
    let use_case = todo_use_case(&app_state);
    let todo = use_case
        .create(auth_user, input)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn update(
    State(app_state): State<AppState>,
    Extension(auth_user): Extension<AuthorizedUser>,
    todo_id: Path<Uuid>,
    Json(body): Json<TodoUpdateRequestBody>,
) -> ApiResult<Json<Todo>> {
    let todo_id = TodoId::from(todo_id.0);
    let input = TodoUpdateInput::try_from(body)?;
    let use_case = todo_use_case(&app_state);
    let updated_todo = use_case
        .update(auth_user, todo_id, input)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(updated_todo))
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
    #[serde(default, deserialize_with = "deserialize_option_split_comma")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoCreateRequestBody {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    #[serde(serialize_with = "serialize_option_date")]
    #[serde(deserialize_with = "deserialize_option_date")]
    pub due_date: Option<Date>,
}

impl TryFrom<TodoCreateRequestBody> for TodoCreateInput {
    type Error = ApiError;

    fn try_from(value: TodoCreateRequestBody) -> Result<Self, Self::Error> {
        Ok(TodoCreateInput {
            title: TodoTitle::new(value.title).map_err(ApiError::from)?,
            description: value
                .description
                .map(TodoDescription::new)
                .transpose()
                .map_err(ApiError::from)?,
            due_date: value.due_date,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoUpdateRequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i16>,
    #[serde(default)]
    #[serde(serialize_with = "serialize_option_date")]
    #[serde(deserialize_with = "deserialize_option_date")]
    pub due_date: Option<Date>,
}

impl TryFrom<TodoUpdateRequestBody> for TodoUpdateInput {
    type Error = ApiError;

    fn try_from(body: TodoUpdateRequestBody) -> Result<Self, Self::Error> {
        Ok(TodoUpdateInput {
            title: body
                .title
                .map(|title| TodoTitle::new(title).map_err(ApiError::from))
                .transpose()?,
            description: body
                .description
                .map(|desc| TodoDescription::new(desc).map_err(ApiError::from))
                .transpose()?,
            status_code: body
                .status_code
                .map(|code| TodoStatusCode::try_from(code).map_err(ApiError::from))
                .transpose()?,
            due_date: body.due_date,
        })
    }
}
