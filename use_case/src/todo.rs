use domain::{
    DomainResult,
    models::Todo,
    repositories::{TodoListInput, TodoRepository},
};

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

    pub async fn list(&self, input: TodoListInput) -> DomainResult<Vec<Todo>> {
        self.todo_repo.list(input).await
    }
}
