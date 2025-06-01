pub mod models;
pub mod password;
pub mod repositories;

use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainErrorKind {
    /// 検証エラー
    Validation,
    /// エンティティが存在しない
    NotFound,
    /// 認証されていない
    Unauthorized,
    /// 禁止された操作
    Forbidden,
    /// リポジトリエラー
    Repository,
    /// 予期しないエラー
    Unexpected,
}

impl std::fmt::Display for DomainErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainErrorKind::Validation => write!(f, "Validation Error"),
            DomainErrorKind::NotFound => write!(f, "Not Found"),
            DomainErrorKind::Unauthorized => write!(f, "Unauthorized"),
            DomainErrorKind::Forbidden => write!(f, "Forbidden"),
            DomainErrorKind::Repository => write!(f, "Repository Error"),
            DomainErrorKind::Unexpected => write!(f, "Unexpected Error"),
        }
    }
}

/// ドメインエラー
#[derive(Debug, thiserror::Error)]
pub struct DomainError {
    pub kind: DomainErrorKind,
    pub messages: Vec<Cow<'static, str>>,
    pub source: anyhow::Error,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DomainError: {} - {:?}", self.kind, self.messages)
    }
}

/// ドメイン結果
pub type DomainResult<T> = Result<T, DomainError>;

fn starts_or_ends_with_whitespace(s: &str) -> bool {
    s.chars().next().is_some_and(|ch| ch.is_whitespace())
        || s.chars().last().is_some_and(|ch| ch.is_whitespace())
}
