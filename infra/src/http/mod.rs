pub mod handler;
pub mod middleware;

use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::{DomainError, DomainErrorKind};

/// API結果
type ApiResult<T> = Result<T, ApiError>;

/// APIエラー
pub struct ApiError {
    /// HTTPステータスコード
    pub status_code: StatusCode,
    /// エラーメッセージ
    pub messages: Vec<Cow<'static, str>>,
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

pub fn internal_server_error<E: std::error::Error>(err: E) -> ApiError {
    ApiError {
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
        messages: vec![err.to_string().into()],
    }
}

pub fn bad_request(message: Cow<'static, str>) -> ApiError {
    ApiError {
        status_code: StatusCode::BAD_REQUEST,
        messages: vec![message],
    }
}

pub fn unauthorized(message: Cow<'static, str>) -> ApiError {
    ApiError {
        status_code: StatusCode::UNAUTHORIZED,
        messages: vec![message],
    }
}

/// クッキーに登録するアクセストークンとリフレッシュトークンのキー
pub const COOKIE_ACCESS_TOKEN_KEY: &str = "access_token";
pub const COOKIE_REFRESH_TOKEN_KEY: &str = "refresh_token";
