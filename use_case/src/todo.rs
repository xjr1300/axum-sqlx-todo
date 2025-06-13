use domain::{
    DomainErrorKind, DomainResult, domain_error,
    models::{Todo, TodoId},
    repositories::{TodoCreateInput, TodoListInput, TodoRepository},
};

use crate::AuthorizedUser;

pub struct TodoUseCase<R>
where
    R: TodoRepository,
{
    pub todo_repo: R,
}

impl<R> TodoUseCase<R>
where
    R: TodoRepository,
{
    pub fn new(todo_repo: R) -> Self {
        Self { todo_repo }
    }

    /// ユーザーのTodoリストを返す。
    pub async fn list(&self, input: TodoListInput) -> DomainResult<Vec<Todo>> {
        self.todo_repo.list(input).await
    }

    /// IDからTodoを取得する。
    ///
    /// 認証されたユーザーが所有するTodoのみを返し、所有していない場合はエラーを返す。
    pub async fn by_id(&self, auth_user: AuthorizedUser, id: TodoId) -> DomainResult<Option<Todo>> {
        let todo = self.todo_repo.by_id(id).await?;
        match todo {
            Some(todo) => {
                if todo.user.id != auth_user.0.id {
                    return Err(domain_error(
                        DomainErrorKind::Forbidden,
                        "You are not authorized to access this todo",
                    ));
                }
                Ok(Some(todo))
            }
            None => Ok(None),
        }
    }

    /// Todoを新規作成する。
    pub async fn create(
        &self,
        auth_user: AuthorizedUser,
        input: TodoCreateInput,
    ) -> DomainResult<Todo> {
        self.todo_repo.create(auth_user.0.id, input).await
    }
}
