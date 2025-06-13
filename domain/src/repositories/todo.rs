use time::{Date, OffsetDateTime};

use crate::{
    DateFilter, DomainErrorKind, DomainResult, NUMERIC_FILTER_MISSING_FROM, NumericOperator,
    domain_error,
    models::{Todo, TodoDescription, TodoId, TodoStatusCode, TodoTitle, UserId},
};

#[async_trait::async_trait]
pub trait TodoRepository {
    /// Todoをリストする。
    async fn list(&self, input: TodoListInput) -> DomainResult<Vec<Todo>>;

    /// Todoを取得する。
    async fn by_id(&self, id: TodoId) -> DomainResult<Option<Todo>>;

    /// Todoを新規作成する。
    async fn create(&self, user_id: UserId, input: TodoCreateInput) -> DomainResult<Todo>;

    /// Todoを更新する。
    async fn update(&self, id: TodoId, todo: TodoUpdateInput) -> DomainResult<Todo>;

    /// Todoを完了する。
    async fn complete(&self, id: TodoId) -> DomainResult<Todo>;

    /// Todoをアーカイブする。
    async fn archive(&self, id: TodoId, archived: bool) -> DomainResult<Todo>;

    /// Todoを削除する
    async fn delete(&self, id: TodoId) -> DomainResult<()>;
}

pub struct TodoListInput {
    /// ユーザーID
    pub user_id: UserId,
    /// キーワード
    pub keyword: Option<String>,
    /// 完了予定日
    pub filter: Option<DateFilter>,
    /// 状態コード
    pub statuses: Option<Vec<TodoStatusCode>>,
}

impl TodoListInput {
    pub fn new(
        user_id: UserId,
        keyword: Option<String>,
        op: Option<NumericOperator>,
        from: Option<Date>,
        to: Option<Date>,
        statuses: Option<Vec<TodoStatusCode>>,
    ) -> DomainResult<Self> {
        if op.is_some() && from.is_none() {
            return Err(domain_error(
                DomainErrorKind::Validation,
                NUMERIC_FILTER_MISSING_FROM,
            ));
        }
        let due_date_filter = op
            .map(|op| DateFilter::new(op, from.unwrap(), to))
            .transpose()?;
        Ok(Self {
            user_id,
            keyword,
            filter: due_date_filter,
            statuses,
        })
    }

    pub fn new_with_user_id(user_id: UserId) -> Self {
        Self {
            user_id,
            keyword: None,
            filter: None,
            statuses: None,
        }
    }
}

pub struct TodoCreateInput {
    /// タイトル
    pub title: TodoTitle,
    /// 説明
    pub description: Option<TodoDescription>,
    /// 完了予定日
    pub due_date: Option<time::Date>,
}

pub struct TodoUpdateInput {
    /// タイトル
    pub title: TodoTitle,
    /// 説明
    pub description: Option<TodoDescription>,
    /// 状態コード
    pub todo_status_code: TodoStatusCode,
    /// 完了予定日
    pub due_date: Option<Date>,
    /// 完了日時
    pub completed_at: Option<OffsetDateTime>,
    /// アーカイブ済み
    pub archived: bool,
}
