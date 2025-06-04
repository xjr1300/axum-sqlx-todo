use domain::{
    DomainResult,
    models::{PHCString, User},
    repositories::{UserInput, UserRepository},
};

pub struct UserUseCase<UR>
where
    UR: UserRepository,
{
    user_repository: UR,
}

impl<UR> UserUseCase<UR>
where
    UR: UserRepository,
{
    /// ユーザー用ユースケースを作成する。
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
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
