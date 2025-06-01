use std::borrow::Cow;

pub mod models;
pub mod password;
pub mod repositories;

/// ドメインエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum DomainError {
    /// 検証エラー
    #[error("{0}")]
    Validation(Cow<'static, str>),

    /// エンティティが存在しない
    #[error("{0} is not found")]
    NotFound(Cow<'static, str>),

    /// 禁止された操作
    #[error("{0}")]
    Forbidden(Cow<'static, str>),

    #[error("{0}")]
    Repository(Cow<'static, str>),

    /// 予期しないエラー
    #[error("{0}")]
    Unexpected(Cow<'static, str>),
}

/// ドメイン結果
pub type DomainResult<T> = Result<T, DomainError>;

fn starts_or_ends_with_whitespace(s: &str) -> bool {
    s.chars().next().is_some_and(|ch| ch.is_whitespace())
        || s.chars().last().is_some_and(|ch| ch.is_whitespace())
}
