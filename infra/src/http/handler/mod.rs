pub mod lookup;
pub mod todo;
pub mod user;

use use_case::{todo::TodoUseCase, user::UserUseCase};

use crate::{
    AppState,
    postgres::repositories::{PgTodoRepository, PgUserRepository},
    redis::token::RedisTokenRepository,
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
    UserUseCase {
        user_repo,
        token_repo,
    }
}

type TodoUseCaseImpl = TodoUseCase<PgTodoRepository>;

fn todo_use_case(app_state: &AppState) -> TodoUseCaseImpl {
    let todo_repo = PgTodoRepository::new(app_state.pg_pool.clone());
    TodoUseCase { todo_repo }
}
