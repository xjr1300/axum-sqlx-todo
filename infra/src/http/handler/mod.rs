pub mod user;

use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use domain::DomainError;

/// APIエラー
pub struct ApiError {
    /// HTTPステータスコード
    status_code: StatusCode,
    /// エラーメッセージ
    messages: Vec<Cow<'static, str>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "messages": self.messages,
        });
        (self.status_code, Json(body)).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::Validation(msg) => ApiError {
                status_code: StatusCode::BAD_REQUEST,
                messages: vec![msg.to_string().into()],
            },
            DomainError::NotFound(msg) => ApiError {
                status_code: StatusCode::NOT_FOUND,
                messages: vec![msg.to_string().into()],
            },
            DomainError::Unauthorized(msg) => ApiError {
                status_code: StatusCode::UNAUTHORIZED,
                messages: vec![msg.to_string().into()],
            },
            DomainError::Forbidden(msg) => ApiError {
                status_code: StatusCode::FORBIDDEN,
                messages: vec![msg.to_string().into()],
            },
            DomainError::Repository(msg) => ApiError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                messages: vec![msg.to_string().into()],
            },
            DomainError::Unexpected(msg) => ApiError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                messages: vec![msg.to_string().into()],
            },
        }
    }
}

/// ヘルスチェックハンドラ
pub async fn health_check() -> &'static str {
    "Ok, the server is running!"
}
