pub mod http;
pub mod postgres;

use settings::AppSettings;

#[derive(Clone)]
pub struct AppState {
    pub app_settings: AppSettings,
    pub pool: sqlx::PgPool,
}
