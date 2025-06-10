use std::{collections::BTreeMap, str::FromStr as _};

use hmac::{Hmac, Mac};
use jwt::{AlgorithmType, Header, SignWithKey as _, Token, VerifyWithKey as _};
use secrecy::{ExposeSecret as _, SecretString};
use sha2::Sha384;
use time::OffsetDateTime;
use uuid::Uuid;

use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{AccessToken, RefreshToken, UserId},
};

const SUBJECT_KEY: &str = "sub";
const EXPIRATION_KEY: &str = "exp";

/// トークンペア
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access: AccessToken,
    pub refresh: RefreshToken,
}

type HmacKey = Hmac<Sha384>;

/// クレイム
#[derive(Debug, Clone, Copy)]
pub struct Claim {
    /// ユーザーID
    pub user_id: UserId,
    /// 有効期限を示すUNIXエポック秒
    pub expiration: u64,
}

/// JWTのアクセストークンとリフレッシュトークンを生成する。
///
/// # 引数
///
/// * `user_id` - ユーザーID
/// * `access_max_age` - アクセストークンの最大有効期間（秒）
/// * `refresh_max_age` - リフレッシュトークンの最大有効期間（秒）
/// * `secret_key` - JWTを作成する秘密鍵
pub fn generate_token_pair(
    user_id: UserId,
    access_expired_at: OffsetDateTime,
    refresh_expired_at: OffsetDateTime,
    secret_key: &SecretString,
) -> DomainResult<TokenPair> {
    // アクセストークンを生成
    let claim = Claim {
        user_id,
        expiration: access_expired_at.unix_timestamp() as u64,
    };
    let access = generate_token(claim, secret_key)?;
    // リフレッシュトークンを生成
    let claim = Claim {
        user_id,
        expiration: refresh_expired_at.unix_timestamp() as u64,
    };
    let refresh = generate_token(claim, secret_key)?;
    Ok(TokenPair {
        access: AccessToken(access),
        refresh: RefreshToken(refresh),
    })
}

/// ユーザーIDと有効期限を指定したJWTを生成する。
///
/// # 引数
///
/// * `claim` - クレイム
/// * `secret_key` - JWTを生成するときの秘密鍵
///
/// # 戻り値
///
/// JWT
pub fn generate_token(claim: Claim, secret_key: &SecretString) -> DomainResult<SecretString> {
    let key: HmacKey = generate_hmac_key(secret_key)?;
    let header = Header {
        algorithm: AlgorithmType::Hs384,
        ..Default::default()
    };
    let mut claims = BTreeMap::new();
    claims.insert(SUBJECT_KEY, claim.user_id.0.to_string());
    claims.insert(EXPIRATION_KEY, claim.expiration.to_string());
    let token = Token::new(header, claims)
        .sign_with_key(&key)
        .map_err(|e| DomainError {
            kind: DomainErrorKind::Unexpected,
            messages: vec![format!("Failed to sign JWT: {e}").into()],
            source: e.into(),
        })?;
    Ok(SecretString::new(token.as_str().into()))
}

fn generate_hmac_key(secret_key: &SecretString) -> DomainResult<HmacKey> {
    Hmac::new_from_slice(secret_key.expose_secret().as_bytes()).map_err(|e| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![format!("{e}").into()],
        source: e.into(),
    })
}

/// JWTからクレイムを取り出す。
///
/// # 引数
///
/// * `token` - JWT
/// * `secret_key` - JWTを生成するときの秘密鍵
///
/// # 戻り値
///
/// クレイム
pub fn retrieve_claim_from_token(
    token: &SecretString,
    secret_key: &SecretString,
) -> DomainResult<Claim> {
    let key: HmacKey = generate_hmac_key(secret_key)?;
    let claims: BTreeMap<String, String> =
        token
            .expose_secret()
            .verify_with_key(&key)
            .map_err(|e| DomainError {
                kind: DomainErrorKind::Unexpected,
                messages: vec!["Failed to verify JWT".into()],
                source: e.into(),
            })?;
    // ユーザーIDを取得
    let user_id = claims.get(SUBJECT_KEY).ok_or_else(|| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec!["The subject was not found in claim".into()],
        source: anyhow::anyhow!("The subject was not found in claim"),
    })?;
    let user_id = Uuid::from_str(user_id).map_err(|e| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec!["The user id was not found in claim".into()],
        source: e.into(),
    })?;
    let user_id = UserId::from(user_id);
    // 有効期限を取得
    let expiration = claims.get(EXPIRATION_KEY).ok_or_else(|| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec!["The expiration was not found in claim".into()],
        source: anyhow::anyhow!("The expiration was not found in claim"),
    })?;
    let expiration = expiration.parse::<u64>().map_err(|e| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![format!("The expiration was not valid in claim: {}", expiration).into()],
        source: e.into(),
    })?;

    Ok(Claim {
        user_id,
        expiration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_generate_valid_token_pair() -> anyhow::Result<()> {
        let requested_at = OffsetDateTime::now_utc();
        let user_id = UserId::from(Uuid::new_v4());
        let access_expired_at = requested_at + Duration::days(1);
        let refresh_expired_at = requested_at + Duration::days(30);
        let secret_key = SecretString::new("super-secret-key".into());

        let token_pair =
            generate_token_pair(user_id, access_expired_at, refresh_expired_at, &secret_key)?;
        let access_claim = retrieve_claim_from_token(&token_pair.access.0, &secret_key)?;
        let refresh_claim = retrieve_claim_from_token(&token_pair.refresh.0, &secret_key)?;

        assert_eq!(access_claim.user_id, user_id);
        assert_eq!(
            access_claim.expiration,
            access_expired_at.unix_timestamp() as u64
        );
        assert_eq!(refresh_claim.user_id, user_id);
        assert_eq!(
            refresh_claim.expiration,
            refresh_expired_at.unix_timestamp() as u64
        );
        Ok(())
    }
}
