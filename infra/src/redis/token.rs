use deadpool_redis::{Connection as RedisConnection, Pool as RedisPool};
use redis::AsyncCommands;
use secrecy::{ExposeSecret, SecretString};

use domain::repositories::{AuthTokenInfo, TokenContent, TokenRepository, divide_auth_token_info};
use domain::{DomainError, DomainErrorKind, DomainResult};

/// Redisトークンリポジトリ
#[derive(Clone)]
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
    async fn register_token_pair<'a>(
        &self,
        access_token_info: &AuthTokenInfo,
        refresh_token_info: &AuthTokenInfo,
    ) -> DomainResult<()> {
        let mut conn = self.connection().await?;
        store(
            &mut conn,
            access_token_info.key.expose_secret(),
            &access_token_info.value,
            access_token_info.max_age,
        )
        .await?;
        store(
            &mut conn,
            refresh_token_info.key.expose_secret(),
            &refresh_token_info.value,
            refresh_token_info.max_age,
        )
        .await?;
        Ok(())
    }

    /// トークンをハッシュ化した文字列からユーザーIDとトークンの種類を取得する。
    ///
    /// # 引数
    ///
    /// * `key` - トークンをハッシュ化した文字列
    ///
    /// # 戻り値
    ///
    /// ユーザーIDとトークンの種類
    async fn get_token_content(&self, key: &SecretString) -> DomainResult<Option<TokenContent>> {
        tracing::trace!("Retrieving token content for key: {}", key.expose_secret());
        let mut conn = self.connection().await?;
        let value = retrieve(&mut conn, key.expose_secret()).await?;
        if value.is_none() {
            return Ok(None);
        }
        let (user_id, token_type) = divide_auth_token_info(&value.unwrap())?;
        Ok(Some(TokenContent {
            user_id,
            token_type,
        }))
    }
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
    tracing::trace!(
        "Storing key: {}, value: {}, max_age: {}",
        key,
        value,
        max_age
    );
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
