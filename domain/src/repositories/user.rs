use time::OffsetDateTime;

use crate::{
    DomainResult,
    models::{Email, FamilyName, GivenName, LoginFailedHistory, PHCString, User, UserId},
};

#[derive(Debug, Clone)]
pub struct UserInput {
    pub family_name: FamilyName,
    pub given_name: GivenName,
    pub email: Email,
}

#[async_trait::async_trait]
pub trait UserRepository {
    /// ユーザーを新規作成する。
    async fn create(&self, user: UserInput, hashed_password: PHCString) -> DomainResult<User>;

    /// ユーザーをIDで取得する。
    async fn by_id(&self, id: UserId) -> DomainResult<Option<User>>;

    /// ユーザーをEメールアドレスで取得する。
    async fn by_email(&self, email: &Email) -> DomainResult<Option<User>>;

    /// ユーザーを更新する。
    async fn update(&self, id: UserId, user: UserInput) -> DomainResult<User>;

    /// ユーザーの最終ログイン日時を更新して、アクセストークンとリフレッシュトークンのキーを保存する。
    /// ユーザーの最終ログイン日時を更新して、アクセストークンとリフレッシュトークンのキーを保存する。
    async fn store_update_last_logged_in_at_and_tokens(
        &self,
        id: UserId,
        logged_in_at: OffsetDateTime,
        access_key: &str,
        access_expired_at: OffsetDateTime,
        refresh_key: &str,
        refresh_expired_at: OffsetDateTime,
    ) -> DomainResult<User>;

    /// ユーザーがログインしたときに生成したアクセストークンとリフレッシュトークンのキーを取得する。
    async fn token_keys_by_id(&self, id: UserId) -> DomainResult<Vec<String>>;

    /// ユーザーがログインしたときに生成したアクセストークンとリフレッシュトークンのキーを削除する。
    async fn delete_token_keys_by_id(&self, id: UserId) -> DomainResult<Vec<String>>;

    /// ユーザーのパスワードを取得する。
    async fn get_hashed_password(&self, id: UserId) -> DomainResult<PHCString>;

    /// ユーザーのパスワードを更新する。
    async fn update_hashed_password(
        &self,
        id: UserId,
        hashed_password: PHCString,
    ) -> DomainResult<()>;

    /// ユーザーを削除する。
    async fn delete(&self, id: UserId) -> DomainResult<()>;

    /// ユーザーのログイン失敗履歴を登録する。
    async fn create_login_failure_history(
        &self,
        user_id: UserId,
        number_of_attempts: i32,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<LoginFailedHistory>;

    /// ユーザーのログイン失敗履歴を取得する。
    async fn get_login_failed_history(
        &self,
        user_id: UserId,
    ) -> DomainResult<Option<LoginFailedHistory>>;

    /// ユーザーのアクティブ状態と、ユーザーの連続ログイン試行回数を更新する。
    async fn increment_number_of_login_attempts(
        &self,
        user_id: UserId,
        max_attempts: u32,
    ) -> DomainResult<()>;

    /// ユーザーのログイン失敗履歴をリセットする。
    ///
    /// 連続ログイン試行回数を1に設定して、最初にログインを試行した日時を指定された日時に更新する。
    async fn reset_login_failure_history(
        &self,
        user_id: UserId,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<()>;
}
