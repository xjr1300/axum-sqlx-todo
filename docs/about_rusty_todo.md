# rusty-todoの説明

## ワークスペース

このプロジェクトは、Rustのワークスペースを使用して構成されています。
ワークスペースを利用して、サードパーティクレートとそのバージョンを管理して、それぞれのワークスペースのメンバークレートが、同じバージョンを使用するようにしています。

## ワークスペースメンバーの依存関係

ワークスペースメンバーの依存関係は、レイヤードアーキテクチャを採用することで、次の通り構成しています。

```text
utils <- domain <- use_case <- infra <- app <- test_suite
```

## アプリケーション設定

Rustには、dotenvファイルを使用して環境変数を設定する安定したクレートが存在しないため、[config](https://crates.io/crates/config)クレートを使用して、tomlファイルに記録されたアプリケーション設定を読み込んでいます。

アプリケーション設定ファイルには、次を設定します。

- `log_level`: ログレベル（`trace`, `debug`, `info`, `warn`, `error`）
- `http`: HTTPサーバー設定
  - `protocol`: プロトコル（`http`, `https`）
  - `host`: ホスト名
  - `port`: ポート番号
- `database`: PostgreSQL設定
  - `host`: ホスト名
  - `port`: ポート番号
  - `user`: ユーザー名
  - `password`: パスワード
  - `name`: データベース名
  - `max_connections`: 最大接続数
  - `connection_timeout`: 接続タイムアウト秒
  - `use_ssl`: SSL/TLS暗号化（`false`, `true`）
- `redis`: Redis設定
  - `host`: ホスト名
  - `port`: ポート番号
- `password`: パスワード設定
  - `min_length`: 最小文字数
  - `max_length`: 最大文字数
  - `symbols`: 使用可能な記号を集めた文字列
  - `max_same_chars`: 同じ文字を使用できる最大文字数
  - `max_repeated_chars`: 連続した文字を使用できる最大文字数
  - `pepper`: ペッパー
  - `hash_memory`: ハッシュ化するときの使用メモリ数（バイト）
  - `hash_iterations`: ハッシュ化の回数
  - `hash_parallelism`: ハッシュ化の並行数
- `login`
  - `attempts_seconds`: 連続ログイン試行許容時間（秒）
  - `max_attempts`: 連続ログイン試行許容回数
- `token`: トークン設定
  - `access_max_age`: アクセストークン有効時間（秒）
  - `refresh_max_age`: リフレッシュトークン有効期間（秒）
  - `jwt_secret`: JWTを生成するときのシークレット

## テレメトリー

[tracing](https://crates.io/crates/tracing)クレートを使用して、処理のテレメトリーをログに出力しています。

## ドメインプリミティブ

エンティティや値オブジェクトの属性の型は、Rust標準の`i32`や`String`を使用せず、それぞれの属性ごとにドメインプリミティブを定義しています。
ドメインプリミティブを定義することで、例えばTodoのタイトルに許容可能な文字数を制限されるようにしています。
なお、許容可能な文字数や数値の範囲などは、[garde](https://crates.io/crates/garde)クレートを使用しています。
また、ドメインプリミティブの実装が煩雑にならないように、宣言的マクロを使用して、ドメインプリミティブの実装を簡略化しています。

```rust
// domain/src/models/todo.rs
#[derive(Debug, Clone, garde::Validate)]
pub struct TodoTitle(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(TodoTitle);
```

## エンティティに対するドメインルールの適用

Todoなどのエンティティに対するドメインルールは、そのエンティテにの唯一のコンストラクタである`new`メソッドで適用しています。
これにより、誤った状態を持つエンティティが構築されることを防止しています。

```rust
// domain/src/models/todo.rs
impl Todo {
    pub fn new(...) -> DomainResult<Self> {
        let todo = Self { ... };
        todo.validate()?;
        Ok(todo)
    }

    fn validate(&self) -> DomainResult<()> {
        //
        // ここでドメインルールを検証する
        //
        Ok(())
    }
}
```

## パスワードの管理

パスワードは、ユーザーが入力したパスワードに、ペッパーとソルトを適用してハッシュ化し、データベースに保存しています。
パスワードのハッシュ化には、[argon2](https://crates.io/crates/argon2)クレートを使用しています。

パスワードにペッパーを適用する際、ペッパーとユーザーが入力したパスワードを1文字ずつ交互に取り出し、文字列を生成します。
ペッパーとパスワードの文字数が異なる場合、文字数が少ない側のすべての文字が取り出された後、多い方の残りのすべての文字列を生成した文字列の末尾に追加します。

[password_hash](https://crates.io/crates/password-hash)クレートと、[argon2](https://crates.io/crates/argon2)クレートを使用して、ソルトの作成、上記で生成した文字列へのソルトの適用、ソルトを適用した文字列のハッシュ化を経て、PHC文字列を生成します。

ここで生成されたPHC文字列がユーザーのパスワードとして記録されます。

## 秘匿する文字列

パスワードやトークンなどがログに出力されないように、[secrecy](https://crates.io/crates/secrecy)クレートを使用して、秘匿する文字列がログなどに出力されないようにしています。

## アクセストークンとリフレッシュトークンの管理

アクセストークンとリフレッシュトークンは、RedisとPostgreSQLで管理します。

### アクセストークンとリフレッシュトークンの有効期限の管理

Redisは、アクセストークンとリフレッシュトークン、それらの有効期限を管理するために使用しています。

なお、ユーザーに対してアクセストークンとリフレッシュトークンを発行する回数に制限はありません。

Redisには、アクセストークンとリフレッシュトークンを、それぞれの有効期限を設定して登録します。
したがって、Redisからアクセストークンまたはリフレッシュトークンを取得できない場合、ユーザーから提示されたそれらのトークンが期限切れであることを示します。

それぞれのトークンとその識別情報は、トークンをハッシュ化した値をキーに、ユーザーIDとトークンの種類を示した文字列（`access`または`refresh`）をカンマで連結した文字列を値にして、Redisに登録されます。

- キー: アクセストークンまたはリフレッシュトークンをSha256でハッシュ化した文字列
- 値: ユーザーIDとトークンの種類を示す文字列をれれカン連結した文字列
  - `2ac90a9c-d267-40b6-ac40-6f0bb3942ef1,access`

### ログアウト後のアクセストークンとリフレッシュトークンの削除

ユーザーがログアウトした後、それまでにログアウトしたユーザーに発行したアクセストークンとリフレッシュトークンを無効にするために、発行したそれらのトークンをPostgreSQLで管理しています。

ユーザーがログアウトしたとき、PostgreSQLからユーザーに発行したトークンを取り出し（同時に削除）、それらトークンをSha256でハッシュ化した文字列を基に、Redisからトークンの識別情報を削除します。

## APIエンドポイントの保護

認証されたユーザーのみアクセスを許可するAPIエンドポイントは、クッキーまたは`Authorization`ヘッダーからアクセストークンを取得して、ロックされていないユーザーであることを確認する`infra.http.middleware.authorized_user_middlewareミドルウェアを使用しています。

例えば次で、認証されていないユーザーが`/users/me`エンドポイントにアクセスすると、`403 Forbidden`が返されます。

```rust
// app/routes/users.rs
pub fn create_user_routes(app_state: AppState) -> Router<AppState> {
    // アクセストークンを要求しないエンドポイント
    let router = Router::new()
        .route("/sign-up", post(sign_up))
        .route("/login", post(login))
        .route("/refresh-tokens", post(refresh_tokens))
        .with_state(app_state.clone());
    // アクセストークンを要求するエンドポイント
    let protected_router = Router::new()
        .route("/me", get(me).patch(update))
        .route("/logout", post(logout))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            authorized_user_middleware,
        ))
        .with_state(app_state);
    // ユーザーに関連するAPIエンドポイントをマージ
    Router::new().merge(router).merge(protected_router)
}
```

## テスト

### 単体テスト

単体テストは、テスト対象コード（SUT）が実装されたモジュール内に、`tests`モジュールを定義して、そこにテストコードを記述しています。
また、パラメーターテストを行うために、[rstest](https://crates.io/crates/rstest)クレートを使用しています。

```rust
// domain/src/models/users.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case("Valid1@Password", "Valid1@Password")]
    #[case(" Valid1@Password", "Valid1@Password")]
    #[case("Valid1@Password ", "Valid1@Password")]
    fn test_raw_password_ok(#[case] password: &str, #[case] expected: &str) -> anyhow::Result<()> {
        let raw_password = RawPassword::new(SecretString::new(password.into()))?;
        assert_eq!(raw_password.0.expose_secret(), expected);
        Ok(())
    }
}
```

単体テストは、つぎの通り実行します。

```sh
cargo test
```

### 統合テスト

統合テストは、`test_suite`クレートに実装しています。

統合テストでは、テストケースごとにテスト用のアプリケーションを起動して、HTTPリクエストを送信して、レスポンスを検証しています。
統合テストのフレームワークは、`test_suite.test_case.TestCase`に実装してあります。

`TestCase`は、テスト開始前に次を行います。

- テスト用のデータベースの作成とマイグレーションの実行
- 使用されていない任意のポート番号をリッスンするテスト用Webアプリの実行

`TestCase`は、テスト終了後に次を行います。

- テスト用Webアプリの正常終了（graceful shutdown）

`TestCase`を使用した統合テストのスケルトンを次にしめします。

```rust
#[tokio::test]
async fn integration_test_case_skeleton() {
    // Initialize the test case
    let test_case = TestCase::begin(true).await;

    /************************************************************

            Implement integration test logic here

    *************************************************************/

    // Terminate the test case gracefully
    test_case.end().await;
}
```

統合テストには、`#[ignore]`属性を付与しています。
したがって、統合テストは次の通り実行します。

```sh
cargo test -- --ignored
```

`TestCase`がテスト用に作成したデータベースは、統合テストが終了しても削除されません。
したがって、統合テスト終了後は、`bin/drop_test_dbs.sh`を実行してテスト用データベースを削除するか、統合テスト実行後に`bin/drop_test_dbs.sh`を実行する`bin/integration_tests.sh`で統合テストを実行してください。
