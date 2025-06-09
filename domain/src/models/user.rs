use std::collections::HashMap;
use std::hash::Hash;

use garde::Validate as _;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use utils::serde::{deserialize_option_offset_datetime, serialize_option_offset_datetime};

use super::primitives::Id;
use crate::{
    DomainError, DomainErrorKind, DomainResult, impl_int_primitive, impl_string_primitive,
    models::primitives::{Description, DisplayOrder},
    starts_or_ends_with_whitespace,
};

/// ユーザーID
pub type UserId = Id<User>;

impl Copy for UserId {}
impl PartialEq for UserId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for UserId {}
impl Hash for UserId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

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

/// パスワード最小文字数
pub const PASSWORD_MIN_LENGTH: usize = 8;
/// パスワード最大文字数
pub const PASSWORD_MAX_LENGTH: usize = 32;
/// パスワードに含めるシンボルの候補
const PASSWORD_SYMBOLS_CANDIDATES: &str = r#"~`!@#$%^&*()_-+={[}]|\:;"'<,>.?/"#;
/// パスワードに同じ文字を含められる文字数
const PASSWORD_MAX_NUMBER_OF_SAME_CHAR: u64 = 3;

/// 未加工のパスワード
#[derive(Debug, Clone)]
pub struct RawPassword(pub SecretString);

/// PHC文字列
///
/// PHC(Password Hashing Competition)文字列は、パスワードのハッシュを表現するための標準形式である。
/// PHC文字列は次の形式で、`$`で区切られた次のパーツで構成される。
///
/// ```text
/// $argon2id$v=19$m=65536,t=2,p=1$Y2F0$E2r1h/6vl6eEuNTmdNG49w
/// ```
///
/// - `$argon2id$` - 使用するハッシュアルゴリズム
/// - `v=19` - バージョン番号
/// - `m=65536,t=2,p=1` - ハッシュのパラメータ（メモリ=64MiB, タイムコスト=2, 並列度=1）
/// - `Y2F0` - ソルト（Base64エンコードされた値）
/// - `E2r1h/6vl6eEuNTmdNG49w` - ハッシュ値（Base64エンコードされた値）
#[derive(Debug, Clone)]
pub struct PHCString(pub SecretString);

impl RawPassword {
    pub fn new(value: SecretString) -> DomainResult<Self> {
        // 文字列の前後の空白をトリム
        let value = value.expose_secret();
        let value = if starts_or_ends_with_whitespace(value) {
            value.trim().to_string()
        } else {
            value.to_string()
        };
        // パスワードの長さを確認
        if value.is_empty() || !(PASSWORD_MIN_LENGTH..=PASSWORD_MAX_LENGTH).contains(&value.len()) {
            let message = format!(
                "The password length must be between {} and {} characters",
                PASSWORD_MIN_LENGTH, PASSWORD_MAX_LENGTH
            );
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.clone().into()],
                source: anyhow::anyhow!(message),
            });
        }
        // 大文字のアルファベットが含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_uppercase()) {
            let message = "The password must contain an uppercase letter";
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.into()],
                source: anyhow::anyhow!(message),
            });
        }
        // 小文字のアルファベットが含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_lowercase()) {
            let message = "The password must contain an lowercase letter";
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.into()],
                source: anyhow::anyhow!(message),
            });
        }
        // 数字が含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_digit()) {
            let message = "The password must contain a digit";
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.into()],
                source: anyhow::anyhow!(message),
            });
        }
        // シンボルが含まれるか確認
        if !value
            .chars()
            .any(|ch| PASSWORD_SYMBOLS_CANDIDATES.contains(ch))
        {
            let message = format!(
                "The password must contain a symbol({})",
                PASSWORD_SYMBOLS_CANDIDATES
            );
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.clone().into()],
                source: anyhow::anyhow!(message),
            });
        }
        // 文字の出現回数を確認して、同じ文字が指定された数以上ないか確認
        let mut number_of_chars: HashMap<char, u64> = HashMap::new();
        value.chars().for_each(|ch| {
            *number_of_chars.entry(ch).or_insert(0) += 1;
        });
        let max_number_of_appearances = number_of_chars.values().max().unwrap();
        if PASSWORD_MAX_NUMBER_OF_SAME_CHAR < *max_number_of_appearances {
            let message = format!(
                "Passwords can't contain more than {} identical characters",
                PASSWORD_MAX_NUMBER_OF_SAME_CHAR
            );
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec![message.clone().into()],
                source: anyhow::anyhow!(message),
            });
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

impl PHCString {
    pub fn new(value: SecretString) -> DomainResult<Self> {
        let value = value.expose_secret();
        if value.is_empty() || value.len() > 255 {
            let message = "The length of PHC strings should be less or equal to 255 characters";
            return Err(DomainError {
                kind: DomainErrorKind::Unexpected,
                messages: vec![message.into()],
                source: anyhow::anyhow!(message),
            });
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

/// アクセストークン
#[derive(Debug, Clone)]
pub struct AccessToken(pub SecretString);

/// リフレッシュトークン
#[derive(Debug, Clone)]
pub struct RefreshToken(pub SecretString);

/// ユーザー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// ID
    pub id: UserId,
    /// 苗字
    pub family_name: FamilyName,
    /// 名前
    pub given_name: GivenName,
    /// Eメールアドレス
    pub email: Email,
    /// ロール
    pub role: Role,
    /// アクティブフラグ
    pub active: bool,
    /// 最終ログイン日時
    #[serde(serialize_with = "serialize_option_offset_datetime")]
    #[serde(deserialize_with = "deserialize_option_offset_datetime")]
    pub last_login_at: Option<OffsetDateTime>,
    /// 作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// 更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// ログイン失敗履歴
///
/// 連続ログイン試行許容時間内に、ログインに失敗した回数を記録する。
#[derive(Debug, Clone, Copy)]
pub struct LoginFailedHistory {
    /// ユーザーID
    pub user_id: UserId,
    /// 最初に試行に失敗した日時
    pub attempted_at: OffsetDateTime,
    /// 試行回数
    pub number_of_attempts: u32,
    /// 作成日時
    pub created_at: OffsetDateTime,
    /// 更新日時
    pub updated_at: OffsetDateTime,
}

/// ロールコード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, garde::Validate)]
pub struct RoleCode(#[garde(range(min = 1, max=i16::MAX))] pub i16);
impl_int_primitive!(RoleCode, i16);

/// ロール名
#[derive(Debug, Clone, garde::Validate)]
pub struct RoleName(#[garde(length(chars, min = 1, max = 50))] pub String);
impl_string_primitive!(RoleName);

/// ロール説明
#[derive(Debug, Clone, garde::Validate)]
pub struct RoleDescription(#[garde(length(chars, min = 1, max = 255))] pub String);
impl_string_primitive!(RoleDescription);

/// ロール
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    /// コード
    pub code: RoleCode,
    /// 名称
    pub name: RoleName,
    /// 説明
    pub description: Option<Description>,
    /// 表示順
    pub display_order: DisplayOrder,
    /// 作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// 更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// 管理者ロールコード
pub const ADMIN_ROLE_CODE: i16 = 1;
/// ユーザーロールコード
pub const USER_ROLE_CODE: i16 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case("Valid1@Password", "Valid1@Password")]
    #[case(" Valid1@Password", "Valid1@Password")]
    #[case("Valid1@Password ", "Valid1@Password")]
    fn test_raw_password_ok(#[case] password: &str, #[case] expected: &str) -> anyhow::Result<()> {
        let raw_password = RawPassword::new(SecretString::new(password.into()))?;
        assert_eq!(raw_password.0.expose_secret(), expected);
        Ok(())
    }

    #[rstest::rstest]
    #[case("Ab1@abc", "length")]
    #[case("Ab1@abcdefghijklmnopqrstuvwxyz012", "length")]
    #[case("valid1@password", "uppercase")]
    #[case("VALID1@PASSWORD", "lowercase")]
    #[case("Valid#@Password", "digit")]
    #[case("Valid12Password", "symbol")]
    #[case("Valid1@Passwordss", "identical")]
    fn test_raw_password_fail(#[case] password: &str, #[case] message: &str) -> anyhow::Result<()> {
        let result = RawPassword::new(SecretString::new(password.into()));
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains(message));
        } else {
            panic!("Expected DomainError::Validation");
        }
        Ok(())
    }
}
