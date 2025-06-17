use sqlx::PgPool;
use time::OffsetDateTime;

use domain::{
    DomainError, DomainResult,
    models::{
        Role, RoleCode, RoleName, TodoStatus, TodoStatusCode, TodoStatusName,
        primitives::{Description, DisplayOrder},
    },
    repositories::LookupRepository,
};

use crate::postgres::repositories::repository_error;

pub struct PgRoleRepository {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl LookupRepository for PgRoleRepository {
    type Entity = Role;
    type Code = RoleCode;

    async fn list(&self) -> DomainResult<Vec<Self::Entity>> {
        sqlx::query_as!(
            RoleRow,
            r#"
            SELECT code, name, description, display_order, created_at, updated_at
            FROM roles
            ORDER BY display_order
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)?
        .into_iter()
        .map(Role::try_from)
        .collect::<Result<Vec<_>, _>>()
    }

    async fn by_code(&self, code: &Self::Code) -> DomainResult<Option<Self::Entity>> {
        sqlx::query_as!(
            RoleRow,
            r#"
            SELECT code, name, description, display_order, created_at, updated_at
            FROM roles
            WHERE code = $1
            "#,
            *code as i16
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?
        .map(Role::try_from)
        .transpose()
    }
}

struct RoleRow {
    code: i16,
    name: String,
    description: Option<String>,
    display_order: i16,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<RoleRow> for Role {
    type Error = DomainError;

    fn try_from(row: RoleRow) -> Result<Self, Self::Error> {
        Ok(Role {
            code: RoleCode::try_from(row.code)?,
            name: RoleName::new(row.name)?,
            description: row.description.map(Description::new).transpose()?,
            display_order: DisplayOrder::new(row.display_order)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub struct PgTodoStatusRepository {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl LookupRepository for PgTodoStatusRepository {
    type Entity = TodoStatus;
    type Code = TodoStatusCode;

    async fn list(&self) -> DomainResult<Vec<Self::Entity>> {
        sqlx::query_as!(
            TodoStatusRow,
            r#"
            SELECT code, name, description, display_order, created_at, updated_at
            FROM todo_statuses
            ORDER BY display_order
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repository_error)?
        .into_iter()
        .map(TodoStatus::try_from)
        .collect::<Result<Vec<_>, _>>()
    }

    async fn by_code(&self, code: &Self::Code) -> DomainResult<Option<Self::Entity>> {
        sqlx::query_as!(
            TodoStatusRow,
            r#"
            SELECT code, name, description, display_order, created_at, updated_at
            FROM todo_statuses
            WHERE code = $1
            "#,
            *code as i16
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?
        .map(TodoStatus::try_from)
        .transpose()
    }
}

struct TodoStatusRow {
    code: i16,
    name: String,
    description: Option<String>,
    display_order: i16,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<TodoStatusRow> for TodoStatus {
    type Error = DomainError;

    fn try_from(row: TodoStatusRow) -> Result<Self, Self::Error> {
        Ok(TodoStatus {
            code: TodoStatusCode::try_from(row.code)?,
            name: TodoStatusName::new(row.name)?,
            description: row.description.map(Description::new).transpose()?,
            display_order: DisplayOrder::new(row.display_order)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
