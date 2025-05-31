use secrecy::{ExposeSecret as _, SecretString};
use serde::Deserialize;

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    /// データベース設定
    pub database: DatabaseSettings,
}

/// データベース設定
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "database")]
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
    pub max_connections: u16,
    /// 接続タイムアウト（秒）
    pub connection_timeout: u64,
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
