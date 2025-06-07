use secrecy::{ExposeSecret as _, SecretString};
use time::OffsetDateTime;
use uuid::Uuid;

use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{
        Email, LoginFailedHistory, PHCString, Role, RoleCode, RoleName, User, UserId,
        primitives::{Description, DisplayOrder},
    },
    repositories::{UserInput, UserRepository},
};

use super::{PgRepository, commit, repository_error};

pub type PgUserRepository = PgRepository<User>;

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    /// ユーザーを新規作成する。
    async fn create(&self, user: UserInput, hashed_password: PHCString) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            WITH inserted AS (
                INSERT INTO users (
                    family_name, given_name, email, hashed_password, active,
                    last_login_at, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                RETURNING
                    id, family_name, given_name, email, role_code,
                    active, last_login_at, created_at, updated_at
            )
            SELECT
                u.id, u.family_name, u.given_name, u.email, u.role_code,
                r.name role_name, r.description role_description, r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at, u.updated_at
            FROM inserted u
            INNER JOIN roles r ON u.role_code = r.code
            "#,
            user.family_name.0,
            user.given_name.0,
            user.email.0,
            hashed_password.0.expose_secret(),
            true,
            None::<OffsetDateTime>,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            let mut e = repository_error(e);
            e.messages
                .push("The email address might already be in use".into());
            e
        })?;
        commit(tx).await?;
        User::try_from(row)
    }

    /// ユーザーをIDで取得する。
    async fn by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            SELECT
                u.id, u.family_name, u.given_name, u.email, u.role_code,
                r.name role_name, r.description role_description, r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at, u.updated_at
            FROM users u
            INNER JOIN roles r ON u.role_code = r.code
            WHERE u.id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?;
        row.map(User::try_from).transpose()
    }

    /// ユーザーをEメールアドレスで取得する。
    async fn by_email(&self, email: &Email) -> DomainResult<Option<User>> {
        let row = sqlx::query_as!(
            UserRow,
            r#"
            SELECT
                u.id, u.family_name, u.given_name, u.email, u.role_code,
                r.name role_name, r.description role_description, r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at, u.updated_at
            FROM users u
            INNER JOIN roles r ON u.role_code = r.code
            WHERE email = $1
            "#,
            email.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?;
        row.map(User::try_from).transpose()
    }

    /// ユーザーを更新する。
    async fn update(&self, id: UserId, user: UserInput) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            WITH updated AS (
                UPDATE users
                SET
                    family_name = $1,
                    given_name = $2,
                    email = $3,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $4
                RETURNING
                    id, family_name, given_name, email, role_code, active,
                    last_login_at, created_at, updated_at
            )
            SELECT
                u.id, u.family_name, u.given_name, u.email, u.role_code,
                r.name role_name, r.description role_description, r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at, u.updated_at
            FROM updated u
            INNER JOIN roles r ON u.role_code = r.code
            "#,
            user.family_name.0,
            user.given_name.0,
            user.email.0,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(repository_error)?;
        match row {
            Some(row) => {
                commit(tx).await?;
                User::try_from(row)
            }
            None => user_not_found(id),
        }
    }

    /// ユーザーの最終ログイン日時を更新する。
    async fn update_last_logged_in_at(
        &self,
        id: UserId,
        logged_in_at: OffsetDateTime,
    ) -> DomainResult<User> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            UserRow,
            r#"
            WITH updated AS (
                UPDATE users
                SET
                    last_login_at = $1,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $2
                RETURNING
                    id, family_name, given_name, email, role_code,
                    active, last_login_at, created_at, updated_at
            )
            SELECT
                u.id, u.family_name, u.given_name, u.email, u.role_code,
                r.name role_name, r.description role_description, r.display_order role_display_order,
                r.created_at role_created_at, r.updated_at role_updated_at,
                u.active, u.last_login_at, u.created_at, u.updated_at
            FROM updated u
            INNER JOIN roles r ON u.role_code = r.code
            "#,
            logged_in_at,
            id.0
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(repository_error)?;
        match row {
            Some(row) => {
                commit(tx).await?;
                User::try_from(row)
            }
            None => user_not_found(id),
        }
    }

    /// ユーザーのパスワードを取得する。
    async fn get_hashed_password(&self, id: UserId) -> DomainResult<PHCString> {
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
        .map_err(repository_error)?;
        match raw_hashed_password {
            Some(raw_hashed_password) => {
                PHCString::new(SecretString::new(raw_hashed_password.into()))
            }
            None => user_not_found(id),
        }
    }

    /// ユーザーのパスワードを更新する。
    async fn update_hashed_password(
        &self,
        id: UserId,
        hashed_password: PHCString,
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
        .map_err(repository_error)?;
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
        .map_err(repository_error)?;
        match affected_rows.rows_affected() {
            0 => user_not_found(id),
            _ => {
                commit(tx).await?;
                Ok(())
            }
        }
    }

    /// ユーザーのログイン失敗履歴を登録する。
    async fn create_login_failure_history(
        &self,
        user_id: UserId,
        number_of_attempts: i32,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<LoginFailedHistory> {
        let mut tx = self.begin().await?;
        let row = sqlx::query_as!(
            LoginFailedHistoryRow,
            r#"
            INSERT INTO login_failed_histories (
                user_id, number_of_attempts, attempted_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            RETURNING
                user_id, number_of_attempts, attempted_at, created_at, updated_at
            "#,
            user_id.0,
            number_of_attempts,
            attempted_at
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(repository_error)?;
        commit(tx).await?;
        Ok(LoginFailedHistory::from(row))
    }

    /// ユーザーのログイン失敗履歴を取得する。
    async fn get_login_failed_history(
        &self,
        user_id: UserId,
    ) -> DomainResult<Option<LoginFailedHistory>> {
        Ok(sqlx::query_as!(
            LoginFailedHistoryRow,
            r#"
            SELECT
                user_id, number_of_attempts, attempted_at, created_at, updated_at
            FROM login_failed_histories
            WHERE user_id = $1
            "#,
            user_id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(repository_error)?
        .map(LoginFailedHistory::from))
    }

    /// ユーザーのログイン試行回数をインクリメントする。
    ///
    /// ユーザーのログイン試行回数をインクリメントして、インクリメント後のログイン試行回数が、最大ログイン試行回数を超えた
    /// 場合は、ユーザーをロックする。
    async fn increment_number_of_login_attempts(
        &self,
        user_id: UserId,
        max_attempts: u32,
    ) -> DomainResult<()> {
        let mut tx = self.begin().await?;
        // ユーザーのログイン試行回数をインクリメント
        sqlx::query!(
            r#"
            UPDATE login_failed_histories
            SET
                number_of_attempts = number_of_attempts + 1,
                updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $1
            "#,
            user_id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(repository_error)?;

        // ユーザーのログイン試行回数が最大ログイン試行回数を超えた場合は、ユーザーをロッユ
        sqlx::query!(
            r#"
            UPDATE users
            SET
                active = FALSE,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
                AND (
                    SELECT number_of_attempts
                    FROM login_failed_histories
                    WHERE user_id = $1
                ) > $2
            "#,
            user_id.0,
            max_attempts as i32,
        )
        .execute(&mut *tx)
        .await
        .map_err(repository_error)?;
        tx.commit().await.map_err(repository_error)?;
        Ok(())
    }

    /// ユーザーのログイン失敗履歴をリセットする。
    ///
    /// 連続ログイン試行回数を1に設定して、最初にログインを試行した日時を指定された日時に更新する。
    async fn reset_login_failure_history(
        &self,
        user_id: UserId,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<()> {
        let mut tx = self.begin().await?;
        let affected_rows = sqlx::query!(
            r#"
            UPDATE login_failed_histories
            SET
                number_of_attempts = 1,
                attempted_at = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $2
            "#,
            attempted_at,
            user_id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(repository_error)?;
        match affected_rows.rows_affected() {
            0 => user_not_found(user_id),
            _ => {
                tx.commit().await.map_err(repository_error)?;
                Ok(())
            }
        }
    }
}

fn user_not_found<T>(id: UserId) -> DomainResult<T> {
    let message = format!("User with id {} not found", id);
    Err(DomainError {
        kind: DomainErrorKind::NotFound,
        messages: vec![message.clone().into()],
        source: anyhow::anyhow!(message),
    })
}

struct UserRow {
    id: Uuid,
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
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.id.into(),
            family_name: row.family_name.try_into()?,
            given_name: row.given_name.try_into()?,
            email: row.email.try_into()?,
            role: Role {
                code: RoleCode(row.role_code),
                name: RoleName::new(row.role_name)?,
                description: row.role_description.map(Description::new).transpose()?,
                display_order: DisplayOrder(row.role_display_order),
                created_at: row.role_created_at,
                updated_at: row.role_updated_at,
            },
            active: row.active,
            last_login_at: row.last_login_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

struct LoginFailedHistoryRow {
    user_id: Uuid,
    number_of_attempts: i32,
    attempted_at: OffsetDateTime,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<LoginFailedHistoryRow> for LoginFailedHistory {
    fn from(row: LoginFailedHistoryRow) -> Self {
        LoginFailedHistory {
            user_id: row.user_id.into(),
            number_of_attempts: row.number_of_attempts as u32,
            attempted_at: row.attempted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
