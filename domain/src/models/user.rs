use garde::Validate as _;
use secrecy::SecretString;
use time::OffsetDateTime;

use super::primitives::Id;
use crate::{DomainError, DomainResult, impl_string_primitive, starts_or_ends_with_whitespace};

/// ユーザーID
pub type UserId = Id<User>;

/// ユーザーの苗字
#[derive(Debug, Clone, garde::Validate)]
pub struct FamilyName(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(FamilyName);

/// ユーザーの名前
#[derive(Debug, Clone, garde::Validate)]
pub struct GivenName(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(GivenName);

/// Eメールアドレス
#[derive(Debug, Clone, garde::Validate)]
pub struct Email(#[garde(email)] pub String);
impl_string_primitive!(Email);

/// ハッシュ化されたパスワード
#[derive(Debug, Clone)]
pub struct HashedPassword(pub SecretString);

impl HashedPassword {
    pub fn new(value: String) -> DomainResult<Self> {
        let value = if starts_or_ends_with_whitespace(&value) {
            value.trim().to_string()
        } else {
            value
        };
        if value.is_empty() || value.len() > 255 {
            return Err(DomainError::Validation("Invalid password length".into()));
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

/// ユーザー
#[derive(Debug, Clone)]
pub struct User {
    /// ID
    pub id: UserId,
    /// 苗字
    pub family_name: FamilyName,
    /// 名前
    pub given_name: GivenName,
    /// Eメールアドレス
    pub email: Email,
    /// アクティブフラグ
    pub active: bool,
    /// 最終ログイン日時
    pub last_login_at: Option<OffsetDateTime>,
    /// 作成日時
    pub created_at: OffsetDateTime,
    /// 更新日時
    pub updated_at: OffsetDateTime,
}

/// ログイン失敗履歴
///
/// 連続ログイン試行許容時間内に、ログインに失敗した回数を記録する。
pub struct LoginFailureHistory {
    /// ユーザーID
    pub user_id: UserId,
    /// 試行回数
    pub number_of_attempts: u32,
    /// 最初に試行に失敗した日時
    pub first_attempted_at: OffsetDateTime,
}
