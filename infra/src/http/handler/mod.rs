pub mod user;

use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use domain::{DomainError, DomainErrorKind};

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
        let status_code = match error.kind {
            DomainErrorKind::Validation => StatusCode::BAD_REQUEST,
            DomainErrorKind::NotFound => StatusCode::NOT_FOUND,
            DomainErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            DomainErrorKind::Forbidden => StatusCode::FORBIDDEN,
            DomainErrorKind::Repository => StatusCode::INTERNAL_SERVER_ERROR,
            DomainErrorKind::Unexpected => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status_code,
            messages: error.messages,
        }
    }
}

/// ヘルスチェックハンドラ
pub async fn health_check() -> &'static str {
    "Ok, the server is running!"
}
