pub mod http;
pub mod jwt;
pub mod password;
pub mod postgres;
pub mod redis;

use settings::AppSettings;

#[derive(Debug, Clone)]
pub struct AppState {
    pub app_settings: AppSettings,
    pub pg_pool: sqlx::PgPool,
    pub redis_pool: deadpool_redis::Pool,
}
