use time::OffsetDateTime;

use crate::{
    DomainResult,
    models::{Email, FamilyName, GivenName, HashedPassword, User, UserId},
};

#[derive(Debug, Clone)]
pub struct UserInput {
    pub family_name: FamilyName,
    pub given_name: GivenName,
    pub email: Email,
    pub active: bool,
}

#[async_trait::async_trait]
pub trait UserRepository {
    /// ユーザーを新規作成する。
    async fn create(&self, user: UserInput, hashed_password: HashedPassword) -> DomainResult<User>;

    /// ユーザーをIDで取得する。
    async fn by_id(&self, id: UserId) -> DomainResult<Option<User>>;

    /// ユーザーを更新する。
    async fn update(&self, id: UserId, user: UserInput) -> DomainResult<User>;

    /// ユーザーの有効状態を更新する。
    async fn update_active(&self, id: UserId, active: bool) -> DomainResult<User>;

    /// ユーザーの最終ログイン日時を更新する。
    async fn update_last_login_at(
        &self,
        id: UserId,
        logged_in_at: OffsetDateTime,
    ) -> DomainResult<User>;

    /// ユーザーのパスワードを取得する。
    async fn get_hashed_password(&self, id: UserId) -> DomainResult<HashedPassword>;

    /// ユーザーのパスワードを更新する。
    async fn update_hashed_password(
        &self,
        id: UserId,
        hashed_password: HashedPassword,
    ) -> DomainResult<()>;

    /// ユーザーを削除する。
    async fn delete(&self, id: UserId) -> DomainResult<()>;
}
