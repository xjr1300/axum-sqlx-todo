use domain::{
    DomainErrorKind, DomainResult, domain_error,
    models::{COMPLETABLE_TODO_STATUS_CODES, Todo, TodoId, TodoStatusCode},
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
        let todo = get_authorized_user_own_todo(&self.todo_repo, &auth_user, todo_id).await?;
        // 完了したTodoまたはアーカイブされたTodoは更新不可
        if todo.status.code == TodoStatusCode::Completed || todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Cannot update completed or archived todo",
            ));
        }
        self.todo_repo.update(todo_id, input).await
    }

    pub async fn complete(&self, auth_user: AuthorizedUser, todo_id: TodoId) -> DomainResult<Todo> {
        // Todoを取得して、認証されたユーザーが所有するTodoが確認
        let todo = get_authorized_user_own_todo(&self.todo_repo, &auth_user, todo_id).await?;
        // 未着手、進行中のTodo以外またはアーカイブされたTodoは完了不可
        if !COMPLETABLE_TODO_STATUS_CODES.contains(&todo.status.code) || todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Only todos with status 'NotStarted' or 'InProgress' can be completed, and archived todos cannot be completed",
            ));
        }
        self.todo_repo.complete(todo_id).await
    }

    pub async fn reopen(
        &self,
        auth_user: AuthorizedUser,
        todo_id: TodoId,
        status: TodoStatusCode,
    ) -> DomainResult<Todo> {
        // 再オープンするときの状態が完了済み出ないことを確認
        if status == TodoStatusCode::Completed {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Cannot reopen a todo with status 'Completed'",
            ));
        }
        // Todoを取得して、認証されたユーザーが所有するTodoが確認
        let todo = get_authorized_user_own_todo(&self.todo_repo, &auth_user, todo_id).await?;
        // Todoの状態が完了済みであることを確認
        if todo.status.code != TodoStatusCode::Completed {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Only completed todos can be reopened",
            ));
        }
        // Todoがアーカイブされていないことを確認
        if todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Archived todos cannot be reopened",
            ));
        }
        self.todo_repo.reopen(todo_id, status).await
    }

    pub async fn archive(
        &self,
        auth_user: AuthorizedUser,
        todo_id: TodoId,
        archived: bool,
    ) -> DomainResult<Todo> {
        // Todoを取得して、認証されたユーザーが所有するTodoが確認
        let todo = get_authorized_user_own_todo(&self.todo_repo, &auth_user, todo_id).await?;
        // アーカイブする場合は、Todoがアーカイブ済みでないこと、アーカイブを解除する場合はTodoがアーカイブ済みであることを確認
        if archived && todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Todo is already archived",
            ));
        } else if !archived && !todo.archived {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "Todo is not archived",
            ));
        }
        self.todo_repo.archive(todo_id, archived).await
    }

    pub async fn delete(&self, auth_user: AuthorizedUser, todo_id: TodoId) -> DomainResult<Todo> {
        let todo = get_authorized_user_own_todo(&self.todo_repo, &auth_user, todo_id).await?;
        self.todo_repo.delete(todo.id).await?;
        Ok(todo)
    }
}

async fn get_authorized_user_own_todo<TR: TodoRepository>(
    todo_repo: &TR,
    auth_user: &AuthorizedUser,
    todo_id: TodoId,
) -> DomainResult<Todo> {
    let todo = todo_repo
        .by_id(todo_id)
        .await?
        .ok_or_else(|| domain_error(DomainErrorKind::NotFound, "Todo not found"))?;
    if todo.user.id != auth_user.0.id {
        return Err(domain_error(
            DomainErrorKind::Forbidden,
            "You are not authorized to update this todo",
        ));
    }
    Ok(todo)
}
