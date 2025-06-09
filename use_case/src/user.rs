use domain::{
    DomainResult,
    models::{PHCString, User},
    repositories::{TokenRepository, UserInput, UserRepository},
};
/// ユーザーユースケース
pub struct UserUseCase<UR, TR>
where
    UR: UserRepository,
    TR: TokenRepository,
{
    /// ユーザーリポジトリ
    pub user_repository: UR,
    /// トークンリポジトリ
    pub token_repository: TR,
}

impl<UR, TR> UserUseCase<UR, TR>
where
    UR: UserRepository,
    TR: TokenRepository,
{
    /// ユーザー用ユースケースを作成する。
    pub fn new(user_repository: UR, token_repository: TR) -> Self {
        Self {
            user_repository,
            token_repository,
        }
    }

    /// ユーザーをサインアップする。
    pub async fn sign_up(
        &self,
        input: UserInput,
        hashed_password: PHCString,
    ) -> DomainResult<User> {
        self.user_repository.create(input, hashed_password).await
    }
}
