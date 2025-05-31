mod todo;
mod user;

pub use todo::*;
pub use user::*;

/// リポジトリコレクション
///
/// 具象リポジトリをDIして使用する。
pub struct Repositories<U, T>
where
    U: UserRepository,
    T: TodoRepository,
{
    /// ユーザーリポジトリ
    pub user_repository: U,
    /// Todoリポジトリ
    pub todo_repository: T,
}
