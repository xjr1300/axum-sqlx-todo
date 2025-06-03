pub mod user;

use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serializer;
use time::{OffsetDateTime, serde::rfc3339};

use domain::{DomainError, DomainErrorKind};

/// API結果
type ApiResult<T> = Result<T, ApiError>;

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

fn serialize_option_offset_datetime<S>(
    dt: &Option<OffsetDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => rfc3339::serialize(dt, serializer),
        _ => unreachable!(),
    }
}

fn serialize_secret_string<S>(s: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(s.expose_secret())
}
