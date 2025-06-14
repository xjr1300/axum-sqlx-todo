use domain::{
    DomainErrorKind, DomainResult, domain_error,
    models::{Todo, TodoId, TodoStatusCode},
    repositories::{TodoCreateInput, TodoListInput, TodoRepository, TodoUpdateInput},
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

    /// Todoを取得する。
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

    /// Todoを更新する。
    ///
    /// 認証されたユーザーが所有するTodoのみを更新できる。
    /// Todoの状態は未着手、進行中、キャンセル、保留のみに変更できる。
    /// それ以外の状態を指定した場合は、エラーを返す。
    /// また、完了したTodo、アーカイブされたTodoは更新できない。
    pub async fn update(
        &self,
        auth_user: AuthorizedUser,
        todo_id: TodoId,
        input: TodoUpdateInput,
    ) -> DomainResult<Todo> {
        // Todoを取得して、認証されたユーザーが所有するTodoが確認
        let todo = self
            .todo_repo
            .by_id(todo_id)
            .await?
            .ok_or_else(|| domain_error(DomainErrorKind::NotFound, "Todo not found"))?;
        if todo.user.id != auth_user.0.id {
            return Err(domain_error(
                DomainErrorKind::Forbidden,
                "You are not authorized to update this todo",
            ));
        }
        // 完了したTodoまたはアーカイブされたTodoは更新不可
        if todo.status.code == TodoStatusCode::Completed || todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Cannot update completed or archived todo",
            ));
        }
        self.todo_repo.update(todo_id, input).await
    }
}
