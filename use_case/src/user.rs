use time::{Duration, OffsetDateTime};

use domain::{
    DomainError, DomainErrorKind, DomainResult,
    models::{AccessToken, Email, LoginFailedHistory, PHCString, RawPassword, RefreshToken, User},
    password::verify_password,
    repositories::{TokenRepository, TokenTtlPair, UserInput, UserRepository},
};
use settings::{LoginSettings, PasswordSettings, TokenSettings};

use crate::jwt::generate_token_pair;

pub struct UserUseCase<UR, TR>
where
    UR: UserRepository,
    TR: TokenRepository,
{
    user_repository: UR,
    token_repository: TR,
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

    /// ユーザーをログインする。
    ///
    /// # 手順
    ///
    /// ## 1. ユーザーの取得
    ///
    /// ユーザーのEメールアドレスから、リポジトリからユーザーを取得する。
    /// リポジトリからユーザーを取得できなかった場合は、404 Not Foundエラーを返す。
    ///
    /// ## 2. ユーザーのアクティブフラグの確認
    ///
    /// ユーザーのアクティブフラグを確認して、アクティブでない場合は、403 Forbiddenエラーを返す。
    ///
    /// ## 3. パスワードの確認
    ///
    /// 次に、ユーザーのパスワードを検証する。
    ///
    /// ### 3.1. パスワードが一致した場合
    ///
    /// ユーザーのパスワードが一致した場合は、最終ログイン日時を更新して、ログイン失敗履歴から、
    /// そのユーザーのレコードを削除する。
    /// その後、200 OKレスポンスとユーザー情報を返す。
    ///
    /// ### 3.2. パスワードが一致しない場合
    ///
    /// ユーザーのログイン試行履歴を確認して、次の通り処理する。
    /// その後、401 Unauthorizedエラーを返す。
    ///
    /// #### 3.2.1. ログイン試行履歴が存在しない場合
    ///
    /// ログイン失敗履歴にそのユーザーのレコードが存在しない場合は、
    /// ログイン失敗履歴に次のレコードを追加する。
    ///
    /// - ログイン失敗回数: 1
    /// - ログイン試行開始日時: 現在の日時
    ///
    ///
    /// #### 3.2.2. ログイン試行履歴が存在する場合
    ///
    /// ログイン失敗履歴にそのユーザーのレコードが存在する場合は、ログイン失敗履歴のログイン試行開始日時を確認する。
    ///
    /// ##### 3.2.2.1. ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間未満の場合
    ///
    /// ```text
    /// 現在日時 - ログイン試行開始日時 < 連続ログイン試行許容時間
    /// ```
    ///
    /// ログイン失敗履歴のログイン試行回数を1回増やす。
    /// そしてログイン試行回数が連続ログイン試行許容回数を超えた場合は、ユーザーのアクティブフラグを無効にする。
    ///
    /// ##### 3.2.2.2. ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間以上の場合
    ///
    /// ```text
    /// 現在日時 - ログイン試行開始日時 >= 連続ログイン試行許容時間
    /// ```
    ///
    /// ログイン試行履歴のログイン試行回数を1に、ログイン試行開始日時を現在の日時に更新する。
    pub async fn login(
        &self,
        input: LoginInput,
        password_settings: &PasswordSettings,
        login_settings: &LoginSettings,
        token_settings: &TokenSettings,
    ) -> DomainResult<LoginOutput> {
        // 現在日時を取得
        let attempted_at = OffsetDateTime::now_utc();
        // Eメールアドレスからユーザーを取得
        let user = self.user_repository.by_email(&input.email).await?;
        if user.is_none() {
            let message = format!("User with email {} not found", input.email);
            return Err(DomainError {
                kind: DomainErrorKind::NotFound,
                messages: vec![message.clone().into()],
                source: anyhow::anyhow!(message),
            });
        }
        let user = user.unwrap();
        // ユーザーのアクティブフラグを確認
        if !user.active {
            let message = format!("User with email {} is not active", input.email);
            return Err(DomainError {
                kind: DomainErrorKind::Forbidden,
                messages: vec![message.clone().into()],
                source: anyhow::anyhow!(message),
            });
        }
        // ユーザーのハッシュからされたPHC文字列を取得
        let hashed_password = self.user_repository.get_hashed_password(user.id).await?;
        // ユーザーのパスワードを検証
        match verify_password(
            &input.raw_password,
            &password_settings.pepper,
            &hashed_password,
        )? {
            true => {
                // ユーザーのパスワードの検証に成功した場合、アクセストークンとリフレッシュトークンを作成
                let access_expiration =
                    attempted_at + Duration::seconds(token_settings.access_expiration as i64);
                let refresh_expiration =
                    attempted_at + Duration::seconds(token_settings.refresh_expiration as i64);
                let token_pair = generate_token_pair(
                    user.id,
                    access_expiration,
                    refresh_expiration,
                    &token_settings.jwt_secret,
                )?;
                // アクセストークンとリフレッシュトークンを、ハッシュ化してRedisに登録
                // Redisには、アクセストークンをハッシュ化した文字列をキーに、ユーザーIDとトークンの種類を表現する文字列を':'で
                // 連結した文字列を値に追加する。
                // Redisに登録するレコードは、トークンの種類別の有効期限を設定する。
                let token_ttl_pair = TokenTtlPair {
                    access: &token_pair.access.0,
                    access_ttl: token_settings.access_expiration,
                    refresh: &token_pair.refresh.0,
                    refresh_ttl: token_settings.refresh_expiration,
                };
                self.token_repository
                    .register_token_pair(user.id, token_ttl_pair)
                    .await?;

                // ユーザーの最終ログイン日時を更新
                self.user_repository
                    .update_last_logged_in_at(user.id, attempted_at)
                    .await?;
                Ok(LoginOutput {
                    access_token: token_pair.access,
                    access_expiration,
                    refresh_token: token_pair.refresh,
                    refresh_expiration,
                })
            }
            false => {
                // ユーザーのログイン失敗履歴を取得
                let failed_history = self
                    .user_repository
                    .get_login_failure_history(user.id)
                    .await?;
                match failed_history {
                    Some(history) => {
                        // ユーザーのログイン失敗履歴が存在する場合
                        history_exists(
                            &self.user_repository,
                            login_settings,
                            &user,
                            history,
                            attempted_at,
                        )
                        .await?;
                    }
                    None => {
                        // ユーザーのログイン失敗履歴が存在しない場合
                        history_does_not_exist(&self.user_repository, &user, attempted_at).await?;
                    }
                }
                Err(DomainError {
                    kind: DomainErrorKind::Unauthorized,
                    messages: vec!["Invalid email or password".into()],
                    source: anyhow::anyhow!("Invalid email or password"),
                })
            }
        }
    }
}

async fn history_exists(
    repo: &impl UserRepository,
    login_settings: &LoginSettings,
    user: &User,
    history: LoginFailedHistory,
    attempted_at: OffsetDateTime,
) -> DomainResult<()> {
    let elapsed_time = attempted_at - history.attempted_at;
    if elapsed_time < Duration::seconds(login_settings.attempts_seconds as i64) {
        /*
        ログインを試行した日時から最初にログインに失敗した日時までの経過時間が、連続ログイン試行許容時間未満の場合、
        ログイン試行回数を1回増やす。
        そして、新しいログイン試行回数が、連続ログイン試行許容回数を超えば場合は、ユーザーのアクティブフラグを無効にする。
         */
        let new_attempts = history.number_of_attempts + 1;
        let new_active = new_attempts <= login_settings.max_attempts;
        repo.update_active_and_number_of_attempts(user.id, new_active, new_attempts as i32)
            .await?;
    } else {
        /*
        ログイン試行開始日時から現在日時までの経過時間が、連続ログイン試行許容時間以上の場合、
        最初にログインを試行した日時をログインを試行した日時に更新して、連続ログイン試行回数を
        1に設定する。
         */
        repo.reset_login_failure_history(user.id, attempted_at)
            .await?;
    }
    Ok(())
}

async fn history_does_not_exist(
    repo: &impl UserRepository,
    user: &User,
    attempted_at: OffsetDateTime,
) -> DomainResult<()> {
    let _ = repo
        .create_login_failure_history(user.id, 1, attempted_at)
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LoginInput {
    pub email: Email,
    pub raw_password: RawPassword,
}

#[derive(Debug, Clone)]
pub struct LoginOutput {
    pub access_token: AccessToken,
    pub access_expiration: OffsetDateTime,
    pub refresh_token: RefreshToken,
    pub refresh_expiration: OffsetDateTime,
}
