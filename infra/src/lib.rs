pub mod http;
pub mod postgres;
pub mod redis;

use settings::AppSettings;

#[derive(Clone)]
pub struct AppState {
    pub app_settings: AppSettings,
    pub pg_pool: sqlx::PgPool,
    pub redis_pool: deadpool_redis::Pool,
}
