use domain::{
    DomainResult,
    models::{PHCString, User, UserId},
    repositories::{TokenRepository, UpdateUserInput, UserInput, UserRepository},
};
/// ユーザーユースケース
pub struct UserUseCase<UR, TR>
where
    UR: UserRepository,
    TR: TokenRepository,
{
    /// ユーザーリポジトリ
    pub user_repo: UR,
    /// トークンリポジトリ
    pub token_repo: TR,
}

impl<UR, TR> UserUseCase<UR, TR>
where
    UR: UserRepository,
    TR: TokenRepository,
{
    /// ユーザー用ユースケースを作成する。
    pub fn new(user_repo: UR, token_repo: TR) -> Self {
        Self {
            user_repo,
            token_repo,
        }
    }

    /// ユーザーをサインアップする。
    pub async fn sign_up(
        &self,
        input: UserInput,
        hashed_password: PHCString,
    ) -> DomainResult<User> {
        self.user_repo.create(input, hashed_password).await
    }

    /// ユーザーを更新する。
    pub async fn update(&self, user_id: UserId, input: UpdateUserInput) -> DomainResult<User> {
        let user = self.user_repo.update(user_id, input).await?;
        Ok(user)
    }
}
