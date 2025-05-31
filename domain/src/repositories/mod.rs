mod todo;
mod user;

pub use todo::*;
pub use user::*;

/// リポジトリコレクション
///
/// 具象リポジトリをDIして使用する。
#[derive(Clone)]
pub struct Repositories<User, Todo>
where
    User: UserRepository + Clone,
    Todo: TodoRepository + Clone,
{
    /// ユーザーリポジトリ
    pub user_repository: User,
    /// Todoリポジトリ
    pub todo_repository: Todo,
}
