use std::str::FromStr as _;

use deadpool_redis::{Connection as RedisConnection, Pool as RedisPool};
use redis::AsyncCommands;
use secrecy::{ExposeSecret as _, SecretString};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use domain::models::UserId;
use domain::repositories::{TokenContent, TokenPairWithExpired, TokenRepository, TokenType};
use domain::{DomainError, DomainErrorKind, DomainResult};

/// Redisトークンリポジトリ
pub struct RedisTokenRepository {
    /// Redis接続プール
    pool: RedisPool,
}

impl RedisTokenRepository {
    /// Redisトークンリポジトリを構築する。
    ///
    /// # 引数
    ///
    /// * `pool` - Redis接続プール
    ///
    /// # 戻り値
    ///
    /// Redis接続プール
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    /// Redisに接続する。
    ///
    /// # 戻り値
    ///
    /// Redis接続
    async fn connection(&self) -> DomainResult<RedisConnection> {
        self.pool.get().await.map_err(|e| DomainError {
            kind: DomainErrorKind::Repository,
            messages: vec!["Failed to connect to the redis".into()],
            source: e.into(),
        })
    }
}

#[async_trait::async_trait]
impl TokenRepository for RedisTokenRepository {
    /// アクセストークンとリフレッシュトークンを登録する。
    ///
    /// # 引数
    ///
    /// * `tokens` - トークンペア
    async fn register_token_pair<'a>(
        &self,
        user_id: UserId,
        tokens: &TokenPairWithExpired<'a>,
        access_max_age: i64,
        refresh_max_age: i64,
    ) -> DomainResult<(String, String)> {
        let access_token_key = generate_key(tokens.access);
        let access_token_value = generate_value(user_id, TokenType::Access);
        let refresh_token_key = generate_key(tokens.refresh);
        let refresh_token_value = generate_value(user_id, TokenType::Refresh);
        let mut conn = self.connection().await?;
        store(
            &mut conn,
            &access_token_key,
            &access_token_value,
            access_max_age as u64,
        )
        .await?;
        store(
            &mut conn,
            &refresh_token_key,
            &refresh_token_value,
            refresh_max_age as u64,
        )
        .await?;
        Ok((access_token_key, refresh_token_key))
    }

    /// トークンからユーザーIDとトークンの種類を取得する。
    ///
    /// # 引数
    ///
    /// * `token` - トークン
    ///
    /// # 戻り値
    ///
    /// ユーザーIDとトークンの種類
    async fn get_token_content(&self, token: &SecretString) -> DomainResult<Option<TokenContent>> {
        let mut conn = self.connection().await?;
        let key = generate_key(token);
        let value = retrieve(&mut conn, &key).await?;
        if value.is_none() {
            return Ok(None);
        }
        let (user_id, token_type) = split_value(&value.unwrap())?;

        Ok(Some(TokenContent {
            user_id,
            token_type,
        }))
    }
}

/// Redisに登録するキーを生成する。
///
/// # 引数
///
/// * `token` - トークン
///
/// # 戻り値
///
/// トークンをハッシュ化した文字列
fn generate_key(token: &SecretString) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.expose_secret().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Redisに登録する値を生成する。
fn generate_value(user_id: UserId, token_type: TokenType) -> String {
    format!("{}:{}", user_id.0, token_type)
}

/// Redisにキーと値を保存する。
///
/// # 引数
///
/// * `conn` - Redisコネクション
/// * `key` - キー
/// * `value` - 値
/// * `max_age` - 生存期間（秒）
async fn store(
    conn: &mut RedisConnection,
    key: &str,
    value: &str,
    max_age: u64,
) -> DomainResult<()> {
    conn.set_ex(key, value, max_age)
        .await
        .map_err(|e| DomainError {
            kind: DomainErrorKind::Repository,
            messages: vec!["Failed to store key and value in redis".into()],
            source: e.into(),
        })
}

/// Redisからキーで値を取得する。
async fn retrieve(conn: &mut RedisConnection, key: &str) -> DomainResult<Option<String>> {
    let value: Option<String> = conn.get(key).await.map_err(|e| DomainError {
        kind: DomainErrorKind::Repository,
        messages: vec!["Failed to retrieve value from redis".into()],
        source: e.into(),
    })?;
    Ok(value)
}

const USER_ID_NOT_FOUND: &str = "The user id was not found in the redis value";
const USER_ID_INVALID: &str = "The user id in the redis value is invalid";
const TOKEN_TYPE_NOT_FOUND: &str = "The token type was not found in the redis value";
const TOKEN_TYPE_INVALID: &str = "The token type in the redis value is invalid";

/// 値をユーザーID、トークンの種類に分離する。
fn split_value(value: &str) -> DomainResult<(UserId, TokenType)> {
    let mut values = value.split(':');
    // ユーザーIDを取得
    let user_id = values.next().ok_or_else(|| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![USER_ID_NOT_FOUND.into()],
        source: anyhow::anyhow!(USER_ID_NOT_FOUND),
    })?;
    let user_id = Uuid::from_str(user_id).map_err(|_| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![USER_ID_INVALID.into()],
        source: anyhow::anyhow!(USER_ID_INVALID),
    })?;
    let user_id = UserId::from(user_id);
    // トークンの種類を取得

    let token_type = values.next().ok_or_else(|| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![TOKEN_TYPE_NOT_FOUND.into()],
        source: anyhow::anyhow!(TOKEN_TYPE_NOT_FOUND),
    })?;
    let token_type = TokenType::try_from(token_type).map_err(|_| DomainError {
        kind: DomainErrorKind::Unexpected,
        messages: vec![TOKEN_TYPE_INVALID.into()],
        source: anyhow::anyhow!(TOKEN_TYPE_INVALID),
    })?;
    Ok((user_id, token_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Redisに登録するユーザーIDとトークンの種類を示す文字列を生成できることを確認
    #[test]
    fn can_generate_user_id_and_token_type_string() -> anyhow::Result<()> {
        let user_id = UserId::default();
        let token_type = TokenType::Access;
        let expected = format!("{}:{}", user_id, token_type);
        let actual = generate_value(user_id, token_type);
        assert_eq!(expected, actual);

        Ok(())
    }

    /// Redisに登録されている文字列の形式を、ユーザーIDとトークンの種類に分割できることを確認
    #[test]
    fn can_split_user_id_and_token_type() -> anyhow::Result<()> {
        let expected_user_id = UserId::default();
        let expected_token_type = TokenType::Refresh;
        let input = format!("{}:{}", expected_user_id, expected_token_type);
        let (user_id, token_type) = split_value(&input)?;
        assert_eq!(expected_user_id, user_id);
        assert_eq!(expected_token_type, token_type);

        Ok(())
    }
}
