use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{Role, Todo, TodoId, TodoStatus, User, UserId, primitives::DisplayOrder},
    repositories::{TodoCreate, TodoRepository, TodoUpdate},
};
use sqlx::PgTransaction;
use time::OffsetDateTime;

use super::{PgRepository, commit, repository_error};

pub type PgTodoRepository = PgRepository<Todo>;

#[derive(Debug, sqlx::FromRow)]
struct TodoRow {
    id: TodoId,
    user_id: UserId,
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
    todo_description: Option<String>,
    todo_display_order: i16,
    todo_created_at: OffsetDateTime,
    todo_updated_at: OffsetDateTime,
    completed_at: Option<OffsetDateTime>,
    archived: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<TodoRow> for Todo {
    type Error = DomainError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let user = User {
            id: row.user_id,
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
            description: row.todo_description.map(|d| d.try_into()).transpose()?,
            display_order: DisplayOrder(row.todo_display_order),
            created_at: row.todo_created_at,
            updated_at: row.todo_updated_at,
        };

        Todo::new(
            row.id,
            user,
            row.title.try_into()?,
            row.description.map(|d| d.try_into()).transpose()?,
            status,
            row.completed_at,
            row.archived,
            row.created_at,
            row.updated_at,
        )
    }
}

#[async_trait::async_trait]
impl TodoRepository for PgTodoRepository {
    // Todoを新規作成する。
    async fn create(&self, todo: TodoCreate) -> DomainResult<Todo> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            WITH inserted AS (
                INSERT INTO todos (
                    user_id, title, description, completed_at, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_description,
                ts.display_order todo_display_order, ts.created_at todo_created_at, ts.updated_at todo_updated_at,
                t.completed_at, t.archived, t.created_at, t.updated_at
            FROM inserted t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            todo.user_id.0,
            todo.title.0,
            todo.description.map(|d| d.0),
            None::<OffsetDateTime> // completed_at is None for new todos
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(repository_error)?;
        todo_commit(tx, row).await
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
                t.todo_status_code, ts.name todo_status_name, ts.description todo_description,
                ts.display_order todo_display_order, ts.created_at todo_created_at, ts.updated_at todo_updated_at,
                t.completed_at, t.archived, t.created_at, t.updated_at
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

    /// Todoを更新する。
    async fn update(&self, id: TodoId, todo: TodoUpdate) -> DomainResult<Todo> {
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
                    completed_at = $4,
                    archived = $5,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $6
                RETURNING
                    id, user_id, title, description, todo_status_code,
                    completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_description,
                ts.display_order todo_display_order, ts.created_at todo_created_at, ts.updated_at todo_updated_at,
                t.completed_at, t.archived, t.created_at, t.updated_at
            FROM updated t
            INNER JOIN users u ON t.user_id = u.id
            INNER JOIN roles r ON u.role_code = r.code
            INNER JOIN todo_statuses ts ON t.todo_status_code = ts.code
            "#,
            todo.title.0,
            todo.description.map(|d| d.0),
            todo.todo_status_code.0,
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
                    completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_description,
                ts.display_order todo_display_order, ts.created_at todo_created_at, ts.updated_at todo_updated_at,
                t.completed_at, t.archived, t.created_at, t.updated_at
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
                    completed_at, archived, created_at, updated_at
            )
            SELECT
                t.id, t.user_id,
                u.family_name, u.given_name, u.email,
                u.role_code, r.name role_name, r.description role_description,r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at user_created_at, u.updated_at user_updated_at,
                t.title, t.description,
                t.todo_status_code, ts.name todo_status_name, ts.description todo_description,
                ts.display_order todo_display_order, ts.created_at todo_created_at, ts.updated_at todo_updated_at,
                t.completed_at, t.archived, t.created_at, t.updated_at
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
