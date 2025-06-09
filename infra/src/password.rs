use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use secrecy::{ExposeSecret as _, SecretString};

use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{PHCString, RawPassword},
};
use settings::PasswordSettings;

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

#[cfg(test)]
mod tests {
    use super::*;

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
