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

macro_rules! pg_lookup_repository {
    ($name:ident, $entity:ty, $code:ty, $code_ty: ty, $row:ty, $table:literal) => {
        pub struct $name {
            pub pool: PgPool,
        }

        #[async_trait::async_trait]
        impl LookupRepository for $name {
            type Entity = $entity;
            type Code = $code;

            async fn list(&self) -> DomainResult<Vec<Self::Entity>> {
                sqlx::query_as::<_, $row>(&format!(
                    r#"
                    SELECT code, name, description, display_order, created_at, updated_at
                    FROM {}
                    ORDER BY display_order
                    "#,
                    $table
                ))
                .fetch_all(&self.pool)
                .await
                .map_err(repository_error)?
                .into_iter()
                .map(<$entity>::try_from)
                .collect::<Result<Vec<_>, _>>()
            }

            async fn by_code(&self, code: &Self::Code) -> DomainResult<Option<Self::Entity>> {
                sqlx::query_as::<_, $row>(&format!(
                    r#"
                    SELECT code, name, description, display_order, created_at, updated_at
                    FROM {}
                    WHERE code = $1
                    "#,
                    $table
                ))
                .bind(*code as $code_ty)
                .fetch_optional(&self.pool)
                .await
                .map_err(repository_error)?
                .map(<$entity>::try_from)
                .transpose()
            }
        }
    };
}

pg_lookup_repository!(PgRoleRepository, Role, RoleCode, i16, RoleRow, "roles");
pg_lookup_repository!(
    PgTodoStatusRepository,
    TodoStatus,
    TodoStatusCode,
    i16,
    TodoStatusRow,
    "todo_statuses"
);

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
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
