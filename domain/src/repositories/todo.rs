use time::OffsetDateTime;

use crate::{
    DomainResult,
    models::{Todo, TodoDescription, TodoId, TodoStatusCode, TodoTitle, UserId},
};

pub struct TodoCreate {
    /// ユーザーID
    pub user_id: UserId,
    /// タイトル
    pub title: TodoTitle,
    /// 説明
    pub description: Option<TodoDescription>,
}

pub struct TodoUpdate {
    pub title: TodoTitle,
    pub description: Option<TodoDescription>,
    pub todo_status_code: TodoStatusCode,
    pub completed_at: Option<OffsetDateTime>,
    pub archived: bool,
}

#[async_trait::async_trait]
pub trait TodoRepository {
    /// Todoを新規作成する。
    async fn create(&self, todo: TodoCreate) -> DomainResult<Todo>;

    /// Todoを取得する。
    async fn by_id(&self, id: TodoId) -> DomainResult<Option<Todo>>;

    /// Todoを更新する。
    async fn update(&self, id: TodoId, todo: TodoUpdate) -> DomainResult<Todo>;

    /// Todoを完了する。
    async fn complete(&self, id: TodoId) -> DomainResult<Todo>;

    /// Todoをアーカイブする。
    async fn archive(&self, id: TodoId, archived: bool) -> DomainResult<Todo>;

    /// Todoを削除する
    async fn delete(&self, id: TodoId) -> DomainResult<()>;
}
