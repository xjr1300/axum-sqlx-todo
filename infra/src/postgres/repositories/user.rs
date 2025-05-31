use secrecy::ExposeSecret as _;

use domain::{
    DomainError, DomainResult,
    models::{HashedPassword, User, UserId},
    repositories::{UserInput, UserRepository},
};
use sqlx::PgTransaction;
use time::OffsetDateTime;

use super::{PgRepository, commit};

struct UserRow {
    id: UserId,
    family_name: String,
    given_name: String,
    email: String,
    active: bool,
    last_login_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.id,
            family_name: row.family_name.try_into()?,
            given_name: row.given_name.try_into()?,
            email: row.email.try_into()?,
            active: row.active,
            last_login_at: row.last_login_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub type PgUserRepository = PgRepository<User>;

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    /// ユーザーを新規作成する。
    async fn create(&self, user: UserInput, hashed_password: HashedPassword) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            INSERT INTO users (
                family_name, given_name, email, hashed_password, active,
                last_login_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            RETURNING
                id, family_name, given_name, email, active, last_login_at,
                created_at, updated_at
            "#,
            user.family_name.0,
            user.given_name.0,
            user.email.0,
            hashed_password.0.expose_secret(),
            user.active,
            None::<OffsetDateTime>,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        user_commit(tx, row).await
    }

    /// ユーザーをIDで取得する。
    async fn by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            SELECT
                id, family_name, given_name, email, active, last_login_at,
                created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        row.map(User::try_from).transpose()
    }

    /// ユーザーを更新する。
    async fn update(&self, id: UserId, user: UserInput) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            UPDATE users
            SET
                family_name = $1,
                given_name = $2,
                email = $3,
                active = $4,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $5
            RETURNING
                id, family_name, given_name, email, active, last_login_at,
                created_at, updated_at
            "#,
            user.family_name.0,
            user.given_name.0,
            user.email.0,
            user.active,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match row {
            Some(row) => user_commit(tx, row).await,
            None => user_not_found(id),
        }
    }

    /// ユーザーの有効状態を更新する。
    async fn update_active(&self, id: UserId, active: bool) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            UPDATE users
            SET
                active = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING
                id, family_name, given_name, email, active, last_login_at,
                created_at, updated_at
            "#,
            active,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match row {
            Some(row) => user_commit(tx, row).await,
            None => user_not_found(id),
        }
    }

    /// ユーザーの最終ログイン日時を更新する。
    async fn update_last_login_at(
        &self,
        id: UserId,
        logged_in_at: OffsetDateTime,
    ) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            UPDATE users
            SET
                last_login_at = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING
                id, family_name, given_name, email, active, last_login_at,
                created_at, updated_at
            "#,
            logged_in_at,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match row {
            Some(row) => user_commit(tx, row).await,
            None => user_not_found(id),
        }
    }

    /// ユーザーのパスワードを取得する。
    async fn get_hashed_password(&self, id: UserId) -> DomainResult<HashedPassword> {
        let raw_hashed_password = sqlx::query_scalar!(
            r#"
            SELECT hashed_password
            FROM users
            WHERE id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match raw_hashed_password {
            Some(hashed_password) => HashedPassword::new(hashed_password),
            None => user_not_found(id),
        }
    }

    /// ユーザーのパスワードを更新する。
    async fn update_hashed_password(
        &self,
        id: UserId,
        hashed_password: HashedPassword,
    ) -> DomainResult<()> {
        let mut tx = self.begin().await?;
        let affected_rows = sqlx::query!(
            r#"
            UPDATE users
            SET hashed_password = $1, updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            "#,
            hashed_password.0.expose_secret(),
            id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match affected_rows.rows_affected() {
            0 => user_not_found(id),
            _ => {
                commit(tx).await?;
                Ok(())
            }
        }
    }

    /// ユーザーを削除する。
    async fn delete(&self, id: UserId) -> DomainResult<()> {
        let mut tx = self.begin().await?;
        let affected_rows = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))?;
        match affected_rows.rows_affected() {
            0 => user_not_found(id),
            _ => {
                commit(tx).await?;
                Ok(())
            }
        }
    }
}

async fn user_commit(tx: PgTransaction<'_>, row: UserRow) -> DomainResult<User> {
    commit(tx).await?;
    User::try_from(row)
}

fn user_not_found<T>(id: UserId) -> DomainResult<T> {
    Err(DomainError::NotFound(
        format!("User with id {} not found", id).into(),
    ))
}
