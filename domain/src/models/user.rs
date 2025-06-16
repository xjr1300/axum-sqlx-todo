use std::collections::HashMap;

use garde::Validate as _;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::OffsetDateTime;

use utils::serde::{deserialize_option_offset_datetime, serialize_option_offset_datetime};

use super::primitives::Id;
use crate::{
    DomainError, DomainErrorKind, DomainResult, domain_error, impl_string_primitive,
    models::primitives::{Description, DisplayOrder},
    starts_or_ends_with_whitespace,
};

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

/// パスワード最小文字数
pub const PASSWORD_MIN_LENGTH: usize = 8;
/// パスワード最大文字数
pub const PASSWORD_MAX_LENGTH: usize = 32;
/// パスワードに含めるシンボルの候補
const PASSWORD_SYMBOLS_CANDIDATES: &str = r#"~!@#$%^&*()_-+={[}]|\:;"'<,>.?/"#;
/// パスワードに同じ文字を含められる文字数
const PASSWORD_MAX_NUMBER_OF_SAME_CHAR: u64 = 3;
/// パスワードに同じ文字が連続して出現できる最大回数
const PASSWORD_MAX_REPEATING_CHARS: u8 = 2;

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
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The password must contain an uppercase letter",
            ));
        }
        // 小文字のアルファベットが含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_lowercase()) {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The password must contain an lowercase letter",
            ));
        }
        // 数字が含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_digit()) {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The password must contain a digit",
            ));
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
        // 文字が連続して出現する回数を確認
        if has_repeating_chars(&value, PASSWORD_MAX_REPEATING_CHARS + 1) {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The password can't contain the same character repeated more than twice",
            ));
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

impl PHCString {
    pub fn new(value: SecretString) -> DomainResult<Self> {
        let value = value.expose_secret();
        if value.is_empty() || value.len() > 255 {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The length of PHC strings should be less or equal to 255 characters",
            ));
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(i16)]
pub enum RoleCode {
    Admin = 1,
    User = 2,
}

impl TryFrom<i16> for RoleCode {
    type Error = DomainError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RoleCode::Admin),
            2 => Ok(RoleCode::User),
            _ => Err(domain_error(
                DomainErrorKind::Validation,
                "Invalid role code",
            )),
        }
    }
}

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

/// 同じ文字が指定された数以上連続して出現するかどうかを確認する。
fn has_repeating_chars(s: &str, max_repeats: u8) -> bool {
    use fancy_regex::Regex;
    let max_repeats = max_repeats - 1;
    let re = Regex::new(&format!(r"(\w)\1{{{},}}", max_repeats)).unwrap();
    re.is_match(s).unwrap()
}

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

    #[rstest::rstest]
    #[case("a", false)]
    #[case("aa", false)]
    #[case("aab", false)]
    #[case("baa", false)]
    #[case("baab", false)]
    #[case("aaa", true)]
    #[case("aaab", true)]
    #[case("abbb", true)]
    #[case("abbba", true)]
    fn test_has_repeating_chars(#[case] s: &str, #[case] expected: bool) {
        assert_eq!(
            has_repeating_chars(s, PASSWORD_MAX_REPEATING_CHARS + 1),
            expected
        );
    }
}
