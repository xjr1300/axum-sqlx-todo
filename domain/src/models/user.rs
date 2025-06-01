use std::collections::HashMap;

use garde::Validate as _;
use secrecy::{ExposeSecret, SecretString};
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
            return Err(DomainError::Validation(
                format!("The password length is greater than or equal to {} characters and less than or equal to {} characters", PASSWORD_MIN_LENGTH, PASSWORD_MAX_LENGTH)
                    .into()));
        }
        // 大文字のアルファベットが含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_uppercase()) {
            return Err(DomainError::Validation(
                "The password must contain an uppercase letter".into(),
            ));
        }
        // 小文字のアルファベットが含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_lowercase()) {
            return Err(DomainError::Validation(
                "The password must contain a lowercase letter".into(),
            ));
        }
        // 数字が含まれるか確認
        if !value.chars().any(|ch| ch.is_ascii_digit()) {
            return Err(DomainError::Validation(
                "The password must contain a digit".into(),
            ));
        }
        // シンボルが含まれるか確認
        if !value
            .chars()
            .any(|ch| PASSWORD_SYMBOLS_CANDIDATES.contains(ch))
        {
            return Err(DomainError::Validation(
                format!(
                    "The password must contain a symbol({})",
                    PASSWORD_SYMBOLS_CANDIDATES
                )
                .into(),
            ));
        }
        // 文字の出現回数を確認して、同じ文字が指定された数以上ないか確認
        let mut number_of_chars: HashMap<char, u64> = HashMap::new();
        value.chars().for_each(|ch| {
            *number_of_chars.entry(ch).or_insert(0) += 1;
        });
        let max_number_of_appearances = number_of_chars.values().max().unwrap();
        if PASSWORD_MAX_NUMBER_OF_SAME_CHAR < *max_number_of_appearances {
            return Err(DomainError::Validation(
                format!(
                    "Passwords can't contain more than {} identical characters",
                    PASSWORD_MAX_NUMBER_OF_SAME_CHAR
                )
                .into(),
            ));
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

impl PHCString {
    pub fn new(value: SecretString) -> DomainResult<Self> {
        let value = value.expose_secret();
        if value.is_empty() || value.len() > 255 {
            return Err(DomainError::Unexpected(
                "The length of PHC strings should be less or equal to 255 characters".into(),
            ));
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
        if let Err(DomainError::Validation(msg)) = result {
            assert!(msg.contains(message));
        } else {
            panic!("Expected DomainError::Validation");
        }
        Ok(())
    }
}
