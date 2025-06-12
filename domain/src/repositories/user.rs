use secrecy::SecretString;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    DomainResult,
    models::{Email, FamilyName, GivenName, LoginFailedHistory, PHCString, User, UserId},
};

#[async_trait::async_trait]
pub trait UserRepository {
    /// ユーザーを新規作成する。
    async fn create(&self, user: UserInput, hashed_password: PHCString) -> DomainResult<User>;

    /// ユーザーをIDで取得する。
    async fn by_id(&self, id: UserId) -> DomainResult<Option<User>>;

    /// ユーザーをEメールアドレスで取得する。
    async fn by_email(&self, email: &Email) -> DomainResult<Option<User>>;

    /// ユーザーを更新する。
    async fn update(&self, id: UserId, user: UpdateUserInput) -> DomainResult<User>;

    /// ユーザーの最終ログイン日時を更新して、認証情報を登録するとともに、ログイン失敗履歴を削除する。
    async fn handle_logged_in(
        &self,
        id: UserId,
        logged_in_at: OffsetDateTime,
        access_key: &SecretString,
        access_expired_at: OffsetDateTime,
        refresh_key: &SecretString,
        refresh_expired_at: OffsetDateTime,
    ) -> DomainResult<()>;

    /// ユーザーがログインしたときに生成したアクセストークンとリフレッシュトークンを取得する。
    async fn user_tokens_by_id(&self, id: UserId) -> DomainResult<Vec<UserToken>>;

    /// ユーザーがログインしたときに生成したアクセストークンとリフレッシュトークンを削除する。
    async fn delete_user_tokens_by_id(&self, id: UserId) -> DomainResult<Vec<SecretString>>;

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

    /// ユーザーのログイン試行回数をインクリメントする。
    ///
    /// ユーザーのログイン試行回数をインクリメントして、インクリメント後のログイン試行回数が、最大ログイン試行回数を超えた
    /// 場合は、ユーザーをロックする。
    async fn increment_number_of_login_attempts(
        &self,
        user_id: UserId,
        max_attempts: u32,
    ) -> DomainResult<()>;

    /// ユーザーのログイン失敗履歴をリセットする。
    ///
    /// 連続ログイン試行回数を1に設定して、最初にログインを試行した日時を指定された日時に更新する。
    async fn reset_login_failed_history(
        &self,
        user_id: UserId,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<()>;
}

#[derive(Debug, Clone)]
pub struct UserInput {
    pub family_name: FamilyName,
    pub given_name: GivenName,
    pub email: Email,
}

#[derive(Debug, Clone)]
pub struct UpdateUserInput {
    pub family_name: Option<FamilyName>,
    pub given_name: Option<GivenName>,
    pub email: Option<Email>,
}

pub struct UserToken {
    pub id: Uuid,
    pub user_id: UserId,
    pub token_key: SecretString,
    pub expired_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
