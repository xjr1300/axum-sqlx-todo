use log::Level as LogLevel;
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Deserializer};
use sqlx::postgres::{PgConnectOptions, PgSslMode};

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct AppSettings {
    /// ログレベル
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: LogLevel,
    /// HTTPサーバー設定
    pub http: HttpSettings,
    /// データベース設定
    pub database: DatabaseSettings,
    /// Redis設定
    pub redis: RedisSettings,
    /// パスワード設定
    pub password: PasswordSettings,
    /// ログイン設定
    pub login: LoginSettings,
    /// トークン設定
    pub token: TokenSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename = "protocol")]
#[serde(rename_all = "lowercase")]
pub enum HttpProtocol {
    /// HTTPプロトコル
    Http,
    /// HTTPSプロトコル
    Https,
}

/// HTTPサーバー設定
#[derive(Debug, Clone, Deserialize)]
pub struct HttpSettings {
    /// プロトコル
    pub protocol: HttpProtocol,
    /// ホスト名
    pub host: String,
    /// ポート番号
    pub port: u16,
}

/// データベース設定
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    /// ホスト名
    pub host: String,
    /// ポート番号
    pub port: u16,
    /// ユーザー名
    pub user: String,
    /// パスワード
    pub password: SecretString,
    /// データベース名
    pub name: String,
    /// 最大接続数
    pub max_connections: u32,
    /// 接続タイムアウト（秒）
    pub connection_timeout: u64,
    /// SSL/TLSを使用するかどうか
    pub use_ssl: bool,
}

/// パスワード設定
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordSettings {
    /// ペッパー
    pub pepper: SecretString,
    /// パスワードをハッシュ化するときのメモリサイズ
    pub hash_memory: u32,
    /// パスワードをハッシュ化するときの反復回数
    pub hash_iterations: u32,
    /// パスワードをハッシュ化するときの並列度
    pub hash_parallelism: u32,
}

/// ログイン設定
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LoginSettings {
    /// 連続ログイン試行許容時間（秒）
    pub attempts_seconds: i64,
    /// 連続ログイン試行許容最大回数（秒）
    pub max_attempts: u32,
}

/// トークン設定
#[derive(Debug, Clone, Deserialize)]
pub struct TokenSettings {
    /// アクセストークンの有効期限（秒）
    pub access_max_age: i64,
    /// リフレッシュトークンの有効期限（秒）
    pub refresh_max_age: i64,
    /// JWTシークレットキー
    pub jwt_secret: SecretString,
}

/// Redis設定
#[derive(Debug, Clone, Deserialize)]
pub struct RedisSettings {
    /// ポート番号
    pub port: u16,
    /// ホスト
    pub host: String,
}

impl HttpSettings {
    /// バインドするアドレス（ホスト名とポート番号）を返す。
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl std::fmt::Display for HttpProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpProtocol::Http => write!(f, "http"),
            HttpProtocol::Https => write!(f, "https"),
        }
    }
}

impl DatabaseSettings {
    /// データベースURIを返す。
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.use_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.user)
            .password(self.password.expose_secret())
            .database(&self.name)
            .ssl_mode(ssl_mode)
    }
}

impl RedisSettings {
    /// RedisURIを返す。
    pub fn uri(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<LogLevel, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    match v.to_lowercase().as_str() {
        "error" => Ok(LogLevel::Error),
        "warn" => Ok(LogLevel::Warn),
        "info" => Ok(LogLevel::Info),
        "debug" => Ok(LogLevel::Debug),
        "trace" => Ok(LogLevel::Trace),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid log level: {}",
            v
        ))),
    }
}
