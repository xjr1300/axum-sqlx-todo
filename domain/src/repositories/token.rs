use std::str::FromStr as _;

use async_trait::async_trait;
use enum_display::EnumDisplay;
use secrecy::{ExposeSecret as _, SecretString};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::UserId;
use crate::{DomainError, DomainErrorKind, DomainResult, domain_error};

/// トークンリポジトリ
#[async_trait]
pub trait TokenRepository: Sync + Send {
    /// アクセストークンとリフレッシュトークンを登録する。
    ///
    /// # 引数
    ///
    /// * `access_token_info` - アクセストークンの情報
    /// * `refresh_token_info` - リフレッシュトークンの情報
    async fn register_token_pair<'a>(
        &self,
        access_token_info: &AuthTokenInfo,
        refresh_token_info: &AuthTokenInfo,
    ) -> DomainResult<()>;

    /// トークンからユーザーIDとトークンの種類を取得する。
    ///
    /// # 引数
    ///
    /// * `token` - トークン
    ///
    /// # 戻り値
    ///
    /// ユーザーIDとトークンの種類
    async fn get_token_content(&self, token: &SecretString) -> DomainResult<Option<TokenContent>>;

    /// 認証情報を削除する。
    async fn delete_token_content(&self, key: &SecretString) -> DomainResult<()>;
}

/// トークンコンテンツ
///
/// アクセストークン及びリフレッシュトークンから取得できる情報を表現する。
#[derive(Debug, Clone, Copy)]
pub struct TokenContent {
    /// ユーザーID
    pub user_id: UserId,
    /// トークンの種類
    pub token_type: TokenType,
}

/// トークンの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumDisplay)]
#[enum_display(case = "Lower")]
pub enum TokenType {
    /// アクセストークン
    Access,
    /// リフレッシュトークン
    Refresh,
}

impl TryFrom<&str> for TokenType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "access" => Ok(Self::Access),
            "refresh" => Ok(Self::Refresh),
            _ => {
                let messages = format!("{value} is not a valid token type");
                Err(DomainError {
                    kind: DomainErrorKind::Validation,
                    messages: vec![messages.clone().into()],
                    source: anyhow::anyhow!(messages),
                })
            }
        }
    }
}

pub struct AuthTokenInfo {
    pub key: SecretString,
    pub value: String,
    pub max_age: u64,
}

/// トークンリポジトリに登録する認証情報のキーと値を生成する。
pub fn generate_auth_token_info(
    user_id: UserId,
    token: &SecretString,
    token_type: TokenType,
    max_age: u64,
) -> AuthTokenInfo {
    AuthTokenInfo {
        key: generate_auth_token_info_key(token),
        value: generate_auth_token_info_value(user_id, token_type),
        max_age,
    }
}

/// トークンリポジトリに登録する認証情報のキーを生成する。
///
/// # 引数
///
/// * `token` - トークン
///
/// # 戻り値
///
/// トークンをハッシュ化した文字列
pub fn generate_auth_token_info_key(token: &SecretString) -> SecretString {
    let mut hasher = Sha256::new();
    hasher.update(token.expose_secret().as_bytes());
    SecretString::new(format!("{:x}", hasher.finalize()).into())
}

/// トークンリポジトリに登録する認証情報の値を生成する。
///
/// # 戻り値
///
/// ユーザーIDとトークンタイプを組み合わせた文字列
fn generate_auth_token_info_value(user_id: UserId, token_type: TokenType) -> String {
    format!("{}:{}", user_id.0, token_type)
}

const USER_ID_NOT_FOUND: &str = "The user id was not found in the redis value";
const USER_ID_INVALID: &str = "The user id in the redis value is invalid";
const TOKEN_TYPE_NOT_FOUND: &str = "The token type was not found in the redis value";
const TOKEN_TYPE_INVALID: &str = "The token type in the redis value is invalid";

/// 認証情報をユーザーIDのとトークンの種類に分割する。
pub fn divide_auth_token_info(value: &str) -> DomainResult<(UserId, TokenType)> {
    let mut values = value.split(':');
    // ユーザーIDを取得
    let user_id = values
        .next()
        .ok_or_else(|| domain_error(DomainErrorKind::Unexpected, USER_ID_NOT_FOUND))?;
    let user_id = Uuid::from_str(user_id).map_err(|_| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![USER_ID_INVALID.into()],
        source: anyhow::anyhow!(USER_ID_INVALID),
    })?;
    let user_id = UserId::from(user_id);
    // トークンの種類を取得

    let token_type = values
        .next()
        .ok_or_else(|| domain_error(DomainErrorKind::Unexpected, TOKEN_TYPE_NOT_FOUND))?;
    let token_type = TokenType::try_from(token_type)
        .map_err(|_| domain_error(DomainErrorKind::Unexpected, TOKEN_TYPE_INVALID))?;
    Ok((user_id, token_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Redisに登録するユーザーIDとトークンの種類を示す文字列を生成できることを確認
    #[test]
    fn generate_auth_token_info_value_ok() -> anyhow::Result<()> {
        let user_id = UserId::default();
        let token_type = TokenType::Access;
        let expected = format!("{user_id}:{token_type}");
        let actual = generate_auth_token_info_value(user_id, token_type);
        assert_eq!(expected, actual);
        Ok(())
    }

    /// Redisに登録されている文字列の形式を、ユーザーIDとトークンの種類に分割できることを確認
    #[test]
    fn divide_auth_token_info_ok() -> anyhow::Result<()> {
        let expected_user_id = UserId::default();
        let expected_token_type = TokenType::Refresh;
        let input = format!("{expected_user_id}:{expected_token_type}");
        let (user_id, token_type) = divide_auth_token_info(&input)?;
        assert_eq!(expected_user_id, user_id);
        assert_eq!(expected_token_type, token_type);
        Ok(())
    }
}
