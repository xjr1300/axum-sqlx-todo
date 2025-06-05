use domain::{
    DomainResult,
    models::{PHCString, User, UserId},
    repositories::{TokenRepository, TokenTtlPair, UserInput, UserRepository},
};
use time::{Duration, OffsetDateTime};

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

    /// ログイン情報を記録する。
    pub async fn store_login_info(
        &self,
        user_id: UserId,
        token_ttl_pair: TokenTtlPair<'_>,
        attempted_at: OffsetDateTime,
    ) -> DomainResult<()> {
        // アクセストークンとリフレッシュトークンを登録
        self.token_repository
            .register_token_pair(user_id, token_ttl_pair)
            .await?;
        // ユーザーのログイン日時を更新
        self.user_repository
            .update_last_logged_in_at(user_id, attempted_at)
            .await?;
        Ok(())
    }

    /// ログインに失敗したときの処理をする。
    pub async fn handle_login_failure(
        &self,
        user_id: UserId,
        attempted_at: OffsetDateTime,
        max_attempts: u32,
        attempts_seconds: u32,
    ) -> DomainResult<()> {
        // ユーザーのログイン失敗履歴を取得
        let failed_history = self
            .user_repository
            .get_login_failure_history(user_id)
            .await?;
        match failed_history {
            Some(history) => {
                // ユーザーのログイン失敗履歴が存在する場合
                let elapsed_time = attempted_at - history.attempted_at;
                if elapsed_time < Duration::seconds(attempts_seconds as i64) {
                    /*
                    ログインを試行した日時から最初にログインに失敗した日時までの経過時間が、
                    連続ログイン試行許容時間未満の場合、 ログイン試行回数を1回増やす。
                    そして、新しいログイン試行回数が、連続ログイン試行許容回数を超えば場合は、
                    ユーザーのアクティブフラグを無効にする。
                     */
                    let new_attempts = history.number_of_attempts + 1;
                    let new_active = new_attempts <= max_attempts;
                    self.user_repository
                        .update_active_and_number_of_attempts(
                            user_id,
                            new_active,
                            new_attempts as i32,
                        )
                        .await?;
                } else {
                    /*
                    ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間以上の場合、
                    最初にログインを試行した日時をログインを試行した日時に更新して、連続ログイン試行回数を
                    1に設定する。
                     */
                    self.user_repository
                        .reset_login_failure_history(user_id, attempted_at)
                        .await?;
                }
            }
            None => {
                // ユーザーのログイン失敗履歴が存在しない場合は、そのユーザーのログイン失敗履歴を登録
                let _ = self
                    .user_repository
                    .create_login_failure_history(user_id, 1, attempted_at)
                    .await?;
            }
        }
        Ok(())
    }
}
