use secrecy::{ExposeSecret as _, SecretString};
use serde::Deserialize;

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    /// HTTPサーバー設定
    pub http_server: HttpServerSettings,
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
pub struct HttpServerSettings {
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
    pub database: String,
    /// 最大接続数
    pub max_connections: u32,
    /// 接続タイムアウト（秒）
    pub connection_timeout: u64,
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
    pub attempts_seconds: u32,
    /// 連続ログイン試行許容最大回数（秒）
    pub max_attempts: u32,
}

/// トークン設定
#[derive(Debug, Clone, Deserialize)]
pub struct TokenSettings {
    /// アクセストークンの有効期限（秒）
    pub access_expiration: u64,
    /// リフレッシュトークンの有効期限（秒）
    pub refresh_expiration: u64,
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

impl HttpServerSettings {
    /// バインドするアドレス（ホスト名とポート番号）を返す。
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl DatabaseSettings {
    /// データベースURIを返す。
    pub fn uri(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database
        )
    }
}

impl RedisSettings {
    /// RedisURIを返す。
    pub fn uri(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}
