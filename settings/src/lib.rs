use secrecy::{ExposeSecret as _, SecretString};
use serde::Deserialize;

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    /// HTTPサーバー設定
    pub http_server: HttpServerSettings,
    /// データベース設定
    pub database: DatabaseSettings,
    /// パスワード設定
    pub password: PasswordSettings,
}

/// HTTPサーバー設定
#[derive(Debug, Clone, Deserialize)]
pub struct HttpServerSettings {
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
