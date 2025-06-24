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

/// クッキーに登録するアクセストークンとリフレッシュトークンのキー
pub const COOKIE_ACCESS_TOKEN_KEY: &str = "access_token";
pub const COOKIE_REFRESH_TOKEN_KEY: &str = "refresh_token";

pub fn bad_request(message: Cow<'static, str>) -> ApiError {
    ApiError {
        status_code: StatusCode::BAD_REQUEST,
        messages: vec![message],
    }
}

pub fn not_found(name: &str) -> ApiError {
    ApiError {
        status_code: StatusCode::NOT_FOUND,
        messages: vec![format!("{} not found", name).into()],
    }
}

const LOGIN_FAILED_MESSAGE: &str = "Login failed. Please check your email and password";

pub fn login_failed() -> ApiError {
    ApiError {
        status_code: StatusCode::BAD_REQUEST,
        messages: vec![LOGIN_FAILED_MESSAGE.into()],
    }
}

const USER_CREDENTIALS_INVALID_MESSAGE: &str = "User credentials are invalid or missing";

pub fn unauthorized() -> ApiError {
    ApiError {
        status_code: StatusCode::UNAUTHORIZED,
        messages: vec![USER_CREDENTIALS_INVALID_MESSAGE.into()],
    }
}

const USER_LOCKED_MESSAGE: &str = "User is locked";

pub fn user_locked() -> ApiError {
    ApiError {
        status_code: StatusCode::LOCKED,
        messages: vec![USER_LOCKED_MESSAGE.into()],
    }
}

pub fn internal_server_error<E: std::error::Error>(err: E) -> ApiError {
    ApiError {
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
        messages: vec![err.to_string().into()],
    }
}
