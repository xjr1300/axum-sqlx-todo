pub mod todo;
pub mod user;

use use_case::user::UserUseCase;

use crate::{
    AppState, postgres::repositories::PgUserRepository, redis::token::RedisTokenRepository,
};

/// ヘルスチェックハンドラ
#[tracing::instrument()]
pub async fn health_check() -> &'static str {
    "Ok, the server is running!"
}

type UserUseCaseImpl = UserUseCase<PgUserRepository, RedisTokenRepository>;

fn user_use_case(app_state: &AppState) -> UserUseCaseImpl {
    let user_repo = PgUserRepository::new(app_state.pg_pool.clone());
    let token_repo = RedisTokenRepository::new(app_state.redis_pool.clone());
    UserUseCase::new(user_repo, token_repo)
}
