use sqlx::{PgTransaction, Postgres};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{Role, Todo, TodoId, TodoStatus, User, UserId, primitives::DisplayOrder},
    repositories::{TodoCreateInput, TodoListInput, TodoRepository, TodoUpdateInput},
};

use super::{PgRepository, commit, repository_error};

pub type PgTodoRepository = PgRepository<Todo>;

#[async_trait::async_trait]
impl TodoRepository for PgTodoRepository {
    /// Todoをリストする。
    async fn list(&self, input: TodoListInput) -> DomainResult<Vec<Todo>> {
        let sql = format!(
            r#"
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM todos t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            {}
            ORDER BY t.due_date NULLS LAST, t.updated_at DESC, t.created_at DESC
            "#,
            list_where_clause(&input, "t")
        );
        sqlx::query_as::<Postgres, TodoRow>(sql.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?
            .into_iter()
            .map(Todo::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    /// Todoを取得する。
    async fn by_id(&self, id: TodoId) -> DomainResult<Option<Todo>> {
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM todos t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            WHERE t.id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?;
        row.map(Todo::try_from).transpose()
    }

    // Todoを新規作成する。
    async fn create(&self, user_id: UserId, input: TodoCreateInput) -> DomainResult<Todo> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            WITH inserted AS (
                INSERT INTO todos (
                    user_id, title, description, due_date, completed_at, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    due_date, completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM inserted t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            user_id.0,
            input.title.0,
            input.description.map(|d| d.0),
            input.due_date,
            None::<OffsetDateTime> // completed_at is None for new todos
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(repository_error)?;
        todo_commit(tx, row).await
    }

    /// Todoを更新する。
    async fn update(&self, id: TodoId, todo: TodoUpdateInput) -> DomainResult<Todo> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            WITH updated AS (
                UPDATE todos
                SET
                    title = $1,
                    description = $2,
                    todo_status_code = $3,
                    due_date = $4,
                    completed_at = $5,
                    archived = $6,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $7
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    due_date, completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM updated t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            todo.title.0,
            todo.description.map(|d| d.0),
            todo.todo_status_code as i16,
            todo.due_date,
            todo.completed_at,
            todo.archived,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(repository_error)?;
        match row {
            Some(row) => todo_commit(tx, row).await,
            None => todo_not_found(id),
        }
    }

    /// Todoを完了する。
    async fn complete(&self, id: TodoId) -> DomainResult<Todo> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            WITH updated AS (
                UPDATE todos
                SET
                    completed_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                WHERE
                    id = $1
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    due_date, completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM updated t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(repository_error)?;
        match row {
            Some(row) => todo_commit(tx, row).await,
            None => todo_not_found(id),
        }
    }

    /// Todoをアーカイブする。
    async fn archive(&self, id: TodoId, archived: bool) -> DomainResult<Todo> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            WITH updated AS (
                UPDATE todos
                SET
                    archived = $1,
                    updated_at = CURRENT_TIMESTAMP
                WHERE
                    id = $2
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    due_date, completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_status_description,
                ts.display_order todo_status_display_order, ts.created_at todo_status_created_at, ts.updated_at todo_status_updated_at,
                t.due_date, t.completed_at, t.archived, t.created_at, t.updated_at
            FROM updated t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            archived,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(repository_error)?;
        match row {
            Some(row) => todo_commit(tx, row).await,
            None => todo_not_found(id),
        }
    }

    /// Todoを削除する
    async fn delete(&self, id: TodoId) -> DomainResult<()> {
        let mut tx = self.begin().await?;
        let query_result = sqlx::query!(
            r#"
            DELETE FROM todos
            WHERE id = $1
            "#,
            id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(repository_error)?;
        match query_result.rows_affected() {
            0 => return todo_not_found(id),
            _ => {
                commit(tx).await?;
                Ok(())
            }
        }
    }
}

fn list_where_clause(input: &TodoListInput, todos_table: &str) -> String {
    let mut condition = format!("WHERE {}.user_id = '{}'", todos_table, input.user_id);
    if input.keyword.is_some() {
        condition.push_str(&format!(
            " AND ({0}.title ILIKE '%{1}%' OR {0}.description ILIKE '%{1}%')",
            todos_table,
            input.keyword.as_ref().unwrap()
        ));
    }
    if input.filter.is_some() {
        let due_date_condition = input
            .filter
            .as_ref()
            .unwrap()
            .sql(&format!("{}.due_date", todos_table));
        condition.push_str(&format!(" AND {due_date_condition}"));
    }
    if let Some(statuses) = &input.statuses {
        condition.push_str(&format!(
            " AND {}.todo_status_code IN ({})",
            todos_table,
            statuses
                .iter()
                .map(|s| (*s as i16).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    condition.push(' ');
    condition
}

#[derive(Debug, sqlx::FromRow)]
struct TodoRow {
    id: Uuid,
    user_id: Uuid,
    family_name: String,
    given_name: String,
    email: String,
    role_code: i16,
    role_name: String,
    role_description: Option<String>,
    role_display_order: i16,
    role_created_at: OffsetDateTime,
    role_updated_at: OffsetDateTime,
    active: bool,
    last_login_at: Option<OffsetDateTime>,
    user_created_at: OffsetDateTime,
    user_updated_at: OffsetDateTime,
    title: String,
    description: Option<String>,
    todo_status_code: i16,
    todo_status_name: String,
    todo_status_description: Option<String>,
    todo_status_display_order: i16,
    todo_status_created_at: OffsetDateTime,
    todo_status_updated_at: OffsetDateTime,
    due_date: Option<Date>,
    completed_at: Option<OffsetDateTime>,
    archived: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<TodoRow> for Todo {
    type Error = DomainError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let user = User {
            id: row.user_id.into(),
            family_name: row.family_name.try_into()?,
            given_name: row.given_name.try_into()?,
            email: row.email.try_into()?,
            role: Role {
                code: row.role_code.try_into()?,
                name: row.role_name.try_into()?,
                description: row.role_description.map(|d| d.try_into()).transpose()?,
                display_order: row.role_display_order.try_into()?,
                created_at: row.role_created_at,
                updated_at: row.role_updated_at,
            },
            active: row.active,
            last_login_at: row.last_login_at,
            created_at: row.user_created_at,
            updated_at: row.user_updated_at,
        };
        let status = TodoStatus {
            code: row.todo_status_code.try_into()?,
            name: row.todo_status_name.try_into()?,
            description: row
                .todo_status_description
                .map(|d| d.try_into())
                .transpose()?,
            display_order: DisplayOrder(row.todo_status_display_order),
            created_at: row.todo_status_created_at,
            updated_at: row.todo_status_updated_at,
        };

        Todo::new(
            row.id.into(),
            user,
            row.title.try_into()?,
            row.description.map(|d| d.try_into()).transpose()?,
            status,
            row.due_date,
            row.completed_at,
            row.archived,
            row.created_at,
            row.updated_at,
        )
    }
}

async fn todo_commit(tx: PgTransaction<'_>, row: TodoRow) -> DomainResult<Todo> {
    commit(tx).await?;
    Todo::try_from(row)
}

fn todo_not_found<T>(id: TodoId) -> DomainResult<T> {
    let message = format!("Todo with id {} not found", id);
    Err(DomainError {
        kind: DomainErrorKind::NotFound,
        messages: vec![message.clone().into()],
        source: anyhow::anyhow!(message),
    })
}
