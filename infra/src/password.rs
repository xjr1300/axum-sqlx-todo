use std::collections::HashMap;

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use secrecy::{ExposeSecret as _, SecretString};

use domain::{
    DomainError, DomainErrorKind, DomainResult, domain_error, models::PHCString,
    starts_or_ends_with_whitespace,
};

use crate::settings::PasswordSettings;

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

/// パスワードをハッシュ化して、PHC文字列を生成する。
///
/// # 引数
///
/// * `settings`: パスワード設定
/// * `raw_password`: ハッシュ化されていない生のパスワード
///
/// # 戻り値
///
/// ハッシュ化されたパスワード
pub fn create_hashed_password(
    settings: &PasswordSettings,
    raw_password: &RawPassword,
) -> DomainResult<PHCString> {
    // パスワードにペッパーをふりかけ
    let peppered_password = sprinkle_pepper(&settings.pepper, raw_password);
    // ソルトを生成
    let salt = SaltString::generate(&mut rand::thread_rng());
    // ハッシュ化パラメーターを設定
    let params = Params::new(
        settings.hash_memory,
        settings.hash_iterations,
        settings.hash_parallelism,
        None,
    )
    .map_err(|e| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![format!("Failed to create password hash parameters: {e}").into()],
        source: anyhow::anyhow!(e),
    })?;
    // PHC文字列を生成
    let phc_string = Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(peppered_password.expose_secret().as_bytes(), &salt)
        .map_err(|e| DomainError {
            kind: DomainErrorKind::Unexpected,
            messages: vec![format!("Failed to create a phc string: {e}").into()],
            source: anyhow::anyhow!(e),
        })?;
    Ok(PHCString(SecretString::new(phc_string.to_string().into())))
}

/// パスワードを検証する。
///
/// # 引数
///
/// * `raw_password` - 検証する未加工なパスワード
/// * `pepper` - 未加工なパスワードに振りかけるペッパー
/// * `hashed_password` - ユーザーのパスワードをハッシュ化したPHC文字列
///
/// # 戻り値
///
/// パスワードの検証に成功した場合は`true`、それ以外の場合は`false`
pub fn verify_password(
    raw_password: &RawPassword,
    pepper: &SecretString,
    hashed_password: &PHCString,
) -> DomainResult<bool> {
    // ハッシュ化されたパスワードをPHC文字列からパース
    let expected_password_hash =
        PasswordHash::new(hashed_password.0.expose_secret()).map_err(|e| DomainError {
            kind: DomainErrorKind::Unexpected,
            messages: vec![format!("Failed to parse password hash: {e}").into()],
            source: anyhow::anyhow!(e),
        })?;
    // パスワードにコショウを振りかけ、パスワードを検証
    let expected_password = sprinkle_pepper(pepper, raw_password);
    Ok(Argon2::default()
        .verify_password(
            expected_password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .is_ok())
}

fn sprinkle_pepper(pepper: &SecretString, raw_password: &RawPassword) -> SecretString {
    let pepper = pepper.expose_secret();
    let password = raw_password.0.expose_secret();
    // 交互にペッパーと生のパスワードを結合
    let mut peppered_password = pepper
        .chars()
        .zip(password.chars())
        .flat_map(|(p, r)| vec![p, r])
        .collect::<String>();
    // ペッパーの文字数とパスワードの文字数が異なる場合の処理
    let pepper_chars_count = pepper.chars().count();
    let password_chars_count = password.chars().count();
    match pepper_chars_count.cmp(&password_chars_count) {
        std::cmp::Ordering::Less => {
            // ペッパーの文字数がパスワードの文字数よりも少ない場合、残りのパスワード文字列を追加
            peppered_password.push_str(&password[pepper_chars_count..]);
        }
        std::cmp::Ordering::Greater => {
            // パスワードの文字数がペッパーの文字数よりも少ない場合、残りのペッパー文字列を追加
            peppered_password.push_str(&pepper[password_chars_count..]);
        }
        _ => {}
    }
    // 生成されたペッパー付きパスワードをSecretStringとして返す
    SecretString::new(peppered_password.into())
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

    #[rstest::rstest]
    #[case(SecretString::new("pepper".into()), RawPassword(SecretString::new("abcde".into())),
           SecretString::new("paebpcpdeer".into()))]
    #[case(SecretString::new("pepper".into()), RawPassword(SecretString::new("abcdefg".into())),
           SecretString::new("paebpcpdeerfg".into()))]
    #[case(SecretString::new("pepper".into()), RawPassword(SecretString::new("abcdef".into())),
           SecretString::new("paebpcpdeerf".into()))]
    fn test_sprinkle_pepper(
        #[case] pepper: SecretString,
        #[case] raw_password: RawPassword,
        #[case] expected: SecretString,
    ) {
        let actual = sprinkle_pepper(&pepper, &raw_password);
        assert_eq!(actual.expose_secret(), expected.expose_secret());
    }

    #[test]
    fn test_create_hashed_password_and_verify() -> anyhow::Result<()> {
        let settings = PasswordSettings {
            pepper: SecretString::new("abcdefg".into()),
            hash_memory: 12288,
            hash_iterations: 3,
            hash_parallelism: 1,
        };
        let raw_password = RawPassword(SecretString::new("password123!".into()));
        let hashed_password = create_hashed_password(&settings, &raw_password)?;
        assert!(verify_password(
            &raw_password,
            &settings.pepper,
            &hashed_password
        )?);
        Ok(())
    }
}
