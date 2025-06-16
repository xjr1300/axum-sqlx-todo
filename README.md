# rusty-todo

- [rusty-todo](#rusty-todo)
  - [Todoについて](#todoについて)
  - [ユーザーエンドポイント](#ユーザーエンドポイント)
    - [ユーザー登録](#ユーザー登録)
      - [パスワード](#パスワード)
    - [ログイン](#ログイン)
      - [保護されたAPIへのリクエスト](#保護されたapiへのリクエスト)
    - [ユーザー情報取得](#ユーザー情報取得)
    - [ユーザー情報の更新](#ユーザー情報の更新)
    - [トークンのリフレッシュ](#トークンのリフレッシュ)
    - [ログアウト](#ログアウト)
  - [Todoエンドポイント](#todoエンドポイント)
    - [Todoリストの取得](#todoリストの取得)
    - [Todoの取得](#todoの取得)
    - [Todoの作成](#todoの作成)
    - [Todoの更新](#todoの更新)
    - [Todoの完了](#todoの完了)
    - [Todoの再オープン](#todoの再オープン)
    - [Todoのアーカイブ／アーカイブ解除](#todoのアーカイブアーカイブ解除)
    - [Todoの削除](#todoの削除)

Todoを管理するWeb APIサービスです。

APIは、RESTfulな設計にあまり**従っていません**。

また、クエリサービスの機能をリポジトリが担っているため、ドメイン駆動設計に従っていません。
さらに、データを更新するエンドポイントが、データを帰す場合があるなど、コマンドクエリ分離の原則に**厳密に**従っていません。

## Todoについて

Todoは、タスクを管理するためのアイテムです。
Todoは、次の状態で管理されます。

- 未着手: タスクがまだ開始されていない状態
- 進行中: タスクが現在進行中の状態
- 完了: タスクが完了した状態
- キャンセル: タスクがキャンセルされた状態
- 保留: タスクが保留中の状態

完了したTodoは、属性を更新できません。

また、Todoをアーカイブすることができます。
アーカイブされたTodoは、アクティブな（アーカイブされていない）Todoと異なり（完了したTodoと同様に）、属性を更新できません。

## ユーザーエンドポイント

### ユーザー登録

サービスにユーザーを登録します。

- エンドポイント: `/users/sign-up`
- `Content-Type`: `application/json`
- メソッド: `POST`
- リクエストボディ:
  - `familyName`: ユーザーの苗字
  - `givenName`: ユーザーの名前
  - `email`: ユーザーのEメールアドレス
  - `password`: ユーザーのパスワード

リクエストボディの例:

```json
{
    "familyName": "山田",
    "givenName": "太郎",
    "email": "taro@example.com",
    "password": "P@ssw0rd!"
}
```

成功した場合、`201 Created`を返します。

レスポンスボディの例:

```json
{
    "id": "2bbfea22-0e39-4585-8afa-4256f097731f",
    "family_name": "山田",
    "given_name": "太郎",
    "email": "taro@example.com",
    "role": {
        "code": 2,
        "name": "ユーザー",
        "description": "通常のユーザーとしての役割",
        "displayOrder": 2,
        "createdAt": "2025-06-14T02:58:38.77412Z",
        "updatedAt": "2025-06-14T02:58:38.77412Z"
    },
    "active": true,
    "last_login_at": null,
    "created_at": "2025-06-16T02:15:31.281083Z",
    "updated_at": "2025-06-16T02:15:31.281083Z"
}
```

#### パスワード

パスワードは次の条件をすべて満たす必要があります。

- 8文字以上、32文字以下
- 英字（大文字・小文字）、数字、記号をそれぞれ1文字以上含む
- 記号は次のいずれか: `~!@#$%^&*()_-+={[}]|\:;"'<,>.?/`
- 同じ文字は3文字まで使用可能
- 同じ文字は2回まで連続可能

また、パスワードはハッシュ化されて本サービスに記録されるため、パスワードを復元することはできません。

### ログイン

サービスにログインします。
ログインに成功すると、アクセストークンとリフレッシュトークンをHTTPレスポンスのヘッダに`Set-Cookie`として設定するとともに、レスポンスボディで返します。

- エンドポイント: `/users/login-up`
- `Content-Type`: `application/json`
- メソッド: `POST`
- リクエストボディ:
  - `email`: ユーザーのEメールアドレス
  - `password`: ユーザーのパスワード

リクエストボディの例:

```json
{
    "email": "taro@example.com",
    "password": "P@ssw0rd!"
}
```

成功した場合、`200 OK`を返します。

レスポンスボディの例:

```json
{
    "accessToken": "eyJhbGciOiJIUzM4NCJ9.eyJleHAiOiIxNzUwMDUxMDkzIiwic3ViIjoiMmJiZmVhMjItMGUzOS00NTg1LThhZmEtNDI1NmYwOTc3MzFmIn0._2YGVq8jZkAysE8mWtsGkipO0lD3NTJU0vcBJ8hucJpFF63MZ9OtDh_Dyk-P2INy",
    "accessExpiredAt": "2025-06-16T05:18:13.373627Z",
    "refreshToken": "eyJhbGciOiJIUzM4NCJ9.eyJleHAiOiIxNzUwMTI2NjkzIiwic3ViIjoiMmJiZmVhMjItMGUzOS00NTg1LThhZmEtNDI1NmYwOTc3MzFmIn0.ZApYw-X-HEM9yLBG59lvpdYf3vWUlbuSKEgxgg_nfO3AvFzV5T9Fa0EhOw5FpSyi",
    "refreshExpiredAt": "2025-06-17T02:18:13.373627Z"
}
```

アクセストークンとリフレッシュトークンのクッキーには、それぞれ次の属性を設定します。

- `Key/Value`: `access_token` / アクセストークン、`refresh_token` / リフレッシュトークン
- `Domain`: 本アプリのドメイン
- `Path`: `/`
- `HttpOnly`: JavaScriptからのアクセスを防ぐ
- `Secure`: HTTPS通信時のみ付与
- `SameSite`: `Strict`に設定し、CSRF対策を行う
- `Max-Age`: それぞれのトークンの有効期限に設定

#### 保護されたAPIへのリクエスト

ブラウザで本サービスの保護されたAPIにリクエストする場合、アクセストークンがクッキーに設定されているため、アクセストークンを管理する必要はありません。

ブラウザ以外で本サービスの保護されたAPIにリクエストする場合、`Authorization`ヘッダーに次の通りアクセストークンを設定する必要があります。

```text
Authorization: Bearer <アクセストークン>
```

### ユーザー情報取得

ログインしているユーザーの情報を取得します。

- アクセス保護: あり
- エンドポイント: `/users/me`
- メソッド: `GET`

成功した場合、`200 OK`を返します。

レスポンスボディは、[ユーザー登録](#ユーザー登録)のレスポンスボディと同様です。

### ユーザー情報の更新

ログインしているユーザーの情報を更新します。

- アクセス保護: あり
- エンドポイント: `/users/me`
- メソッド: `PATCH`
- リクエストボディ:
  - `familyName`: ユーザーの苗字、オプション
  - `givenName`: ユーザーの名前、オプション
  - `email`: ユーザーのEメールアドレス、オプション

リクエストボディの例:

```json
{
    "givenName": "次郎"
}
```

成功した場合、`200 OK`を返します。

レスポンスボディは、[ユーザー登録](#ユーザー登録)のレスポンスボディと同様です。

### トークンのリフレッシュ

アクセストークンの有効期限が切れた場合に、リフレッシュトークンを使用して新しいアクセストークンとリフレッシュトークンを取得します。

- アクセス保護: あり
- エンドポイント: `/users/refresh-tokens`
- メソッド: `POST`
- リクエストボディ:
  - `refreshToken`: リフレッシュトークン、オプション

リクエストボディの例:

```json
{
    "refreshToken": "eyJhbGciOiJIUzM4NCJ9.eyJleHAiOiIxNzUwMTI2NjkzIiwic3ViIjoiMmJiZmVhMjItMGUzOS00NTg1LThhZmEtNDI1NmYwOTc3MzFmIn0.ZApYw-X-HEM9yLBG59lvpdYf3vWUlbuSKEgxgg_nfO3AvFzV5T9Fa0EhOw5FpSyi"
}
```

成功した場合、`200 OK`を返します。

レスポンスボディは、[ログイン](#ログイン)のレスポンスボディと同様です。

### ログアウト

本サービスからログアウトして、アクセストークンとリフレッシュトークンを管理するクッキーを無効化します。

- アクセス保護: あり
- エンドポイント: `/users/logout`
- メソッド: `POST`

成功した場合、`204 No Content`を返します。

レスポンスボディは、[ユーザー登録](#ユーザー登録)のレスポンスボディと同様です。

## Todoエンドポイント

### Todoリストの取得

ログインしているユーザーのTodoリストを取得します。

- アクセス保護: あり
- エンドポイント: `/todos`
- メソッド: `GET`

成功した場合、`200 OK`を返します。

レスポンスボディの例:

```json
[
  {
    "id": "4da95cdb-6898-4739-b2be-62ceaa174baf",
    "user": {
      "id": "47125c09-1dea-42b2-a14e-357e59acf3dc",
      "family_name": "山田",
      "given_name": "太郎",
      "email": "taro@example.com",
      "role": {
        "code": 2,
        "name": "ユーザー",
        "description": "通常のユーザーとしての役割",
        "displayOrder": 2,
        "createdAt": "2025-06-16T04:05:00.68412Z",
        "updatedAt": "2025-06-16T04:05:00.68412Z"
      },
      "active": true,
      "last_login_at": "2025-06-16T04:05:00.77916Z",
      "created_at": "2025-06-16T04:05:00.757214Z",
      "updated_at": "2025-06-16T04:05:01.129699Z"
    },
    "title": "チームミーティング",
    "description": "プロジェクトの進捗確認",
    "status": {
      "code": 2,
      "name": "進行中",
      "description": "タスクが現在進行中の状態",
      "display_order": 2,
      "created_at": "2025-06-16T04:05:00.68412Z",
      "updated_at": "2025-06-16T04:05:00.68412Z"
    },
    "due_date": "2025-06-12",
    "completed_at": null,
    "archived": false,
    "created_at": "2025-06-03T00:30:00Z",
    "updated_at": "2025-06-10T05:00:00Z"
  },
  {
    "id": "ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175",
    "user": {
      "id": "47125c09-1dea-42b2-a14e-357e59acf3dc",
      "family_name": "山田",
      "given_name": "太郎",
      "email": "taro@example.com",
      "role": {
        "code": 2,
        "name": "ユーザー",
        "description": "通常のユーザーとしての役割",
        "displayOrder": 2,
        "createdAt": "2025-06-16T04:05:00.68412Z",
        "updatedAt": "2025-06-16T04:05:00.68412Z"
      },
      "active": true,
      "last_login_at": "2025-06-16T04:05:00.77916Z",
      "created_at": "2025-06-16T04:05:00.757214Z",
      "updated_at": "2025-06-16T04:05:01.129699Z"
    },
    "title": "レポート提出",
    "description": "月次レポートを作成して提出",
    "status": {
      "code": 1,
      "name": "未着手",
      "description": "タスクがまだ開始されていない状態",
      "display_order": 1,
      "created_at": "2025-06-16T04:05:00.68412Z",
      "updated_at": "2025-06-16T04:05:00.68412Z"
    },
    "due_date": "2025-06-12",
    "completed_at": null,
    "archived": false,
    "created_at": "2025-06-07T21:30:00Z",
    "updated_at": "2025-06-07T22:00:00Z"
  }
]
```

### Todoの取得

TodoのIDを指定して、Todoを取得します。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}`
- パスパラメータ:
  - `id`: TodoのID
- メソッド: `GET`
- クエリパラメータ
  - `keyword`: オプション、Todoのタイトルや説明に含まれるキーワードでフィルタリングします。
  - `op`: オプション、完了予定日をフィルタリングするときの演算子を指定します。
    - `eq`: 完了予定日が指定した日付と等しい
    - `ne`: 完了予定日が指定した日付と異なるか、完了予定日が指定されていない
    - `gt`: 完了予定日が指定した日付を含まない後の日付
    - `gte`: 完了予定日が指定した日付を含む後の日付
    - `lt`: 完了予定日が指定した日付を含まない前の日付
    - `lte`: 完了予定日が指定した日付を含む前の日付
    - `between`: 完了予定日が指定した日付の範囲内
    - `notBetween`: 完了予定日が指定した日付の範囲外か、完了予定日が指定されていない
    - `isNull`: 完了予定日が指定されていない
    - `isNotNull`: 完了予定日が指定されている
  - `from`: `op`に`isNull`または`isNotNull`を指定したときは必須、`eq`などを指定したときの日付、または`between`などを指定したときの範囲の起点日付をISO8601形式で指定します。
  - `to`: `op`に`between`または`notBetween`を指定したときは必須、範囲の終点日付をISO8601形式で指定します。
  - `statuses`: オプション、フィルタリングするTodoの状態を示す数値をカンマ区切りで指定します。
    - `1`: 未着手
    - `2`: 進行中
    - `3`: 完了
    - `4`: キャンセル
    - `5`: 保留
  - `archived`: オプション、アーカイブされたTodoを取得するか、アクティブな（アーカイブされていない）Todoを取得するか、`true`と`false`で指定します。

リクエストURLの例:

```text
/todos?keyword=ミーティング&op=eq&from=2025-06-01&statuses=1,2
```

成功した場合、`200 OK`を返します。

レスポンスボディの例:

```json
{
    "id": "4da95cdb-6898-4739-b2be-62ceaa174baf",
    "user": {
        "id": "47125c09-1dea-42b2-a14e-357e59acf3dc",
        "family_name": "山田",
        "given_name": "太郎",
        "email": "taro@example.com",
        "role": {
            "code": 2,
            "name": "ユーザー",
            "description": "通常のユーザーとしての役割",
            "displayOrder": 2,
            "createdAt": "2025-06-16T04:05:00.68412Z",
            "updatedAt": "2025-06-16T04:05:00.68412Z"
        },
        "active": true,
        "last_login_at": "2025-06-16T04:05:00.77916Z",
        "created_at": "2025-06-16T04:05:00.757214Z",
        "updated_at": "2025-06-16T04:05:01.129699Z"
    },
    "title": "チームミーティング",
    "description": "プロジェクトの進捗確認",
    "status": {
        "code": 2,
        "name": "進行中",
        "description": "タスクが現在進行中の状態",
        "display_order": 2,
        "created_at": "2025-06-16T04:05:00.68412Z",
        "updated_at": "2025-06-16T04:05:00.68412Z"
    },
    "due_date": "2025-06-12",
    "completed_at": null,
    "archived": false,
    "created_at": "2025-06-03T00:30:00Z",
    "updated_at": "2025-06-10T05:00:00Z"
}
```

### Todoの作成

ログインしているユーザーのTodoを作成します。
作成されたTodoの状態は、未着手です。

- アクセス保護: あり
- エンドポイント: `/todos`
- `Content-Type`: `application/json`
- メソッド: `POST`
- リクエストボディ:
  - `title`: Todoのタイトル
  - `description`: Todoの説明、オプション
  - `dueDate`: Todoの期限日（ISO8601形式）、オプション

リクエストボディの例:

```json
{
    "title": "顧客体験改善プロジェクトの開始",
    "description": "プロジェクトのキックオフミーティングを設定する",
    "dueDate": "2025-06-20"
}
```

成功した場合、`201 Created`を返します。

レスポンスボディは、[Todoの取得](#todoの取得)のレスポンスボディと同様です。

### Todoの更新

ログインしているユーザーのTodoを更新します。

- 完了しているTodo、アーカイブされているTodoは更新できません。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}`
- パスパラメータ:
  - `id`: TodoのID
- `Content-Type`: `application/json`
- メソッド: `PATCH`
- リクエストボディ:
  - `title`: Todoのタイトル、オプション
  - `description`: Todoの説明、オプション
  - `statusCode`: Todoの状態を示す数値、オプション
    - `1`: 未着手
    - `2`: 進行中
    - `3`: 完了
    - `4`: キャンセル
    - `5`: 保留
  - `dueDate`: Todoの期限日（ISO8601形式）、オプション

リクエストボディの例:

```json
{
    "title": "顧客管理サービス開発プロジェクトの開始"
}
```

成功した場合、`200 OK`を返します。

レスポンスボディは、[Todoの取得](#todoの取得)のレスポンスボディと同様です。

### Todoの完了

ログインしているユーザーのTodoを完了します。
Todoを完了すると、自動的にTodoの状態が完了に更新され、`completedAt`に完了日時が設定されます。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}/complete`
- パスパラメータ:
  - `id`: TodoのID
- メソッド: `POST`

成功した場合、`200 OK`を返します。

レスポンスボディは、[Todoの取得](#todoの取得)のレスポンスボディと同様です。

### Todoの再オープン

ログインしているユーザーの完了したTodoを状態を指定して再オープンします。
Todoを再オープンすると、Todoの状態が更新され、`completedAt`が`null`に設定されます。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}/reopen`
- パスパラメータ:
  - `id`: TodoのID
- `Content-Type`: `application/json`
- メソッド: `POST`
- リクエストボディ:
  - `todoStatusCode`: Todoの状態を示す数値
    - `1`: 未着手
    - `2`: 進行中
    - `4`: キャンセル
    - `5`: 保留

リクエストボディの例:

```json
{
    "todoStatusCode": 1
}
```

成功した場合、`200 OK`を返します。

レスポンスボディは、[Todoの取得](#todoの取得)のレスポンスボディと同様です。

### Todoのアーカイブ／アーカイブ解除

ログインしているユーザーのTodoをアーカイブ、またはアーカイブ解除してアクティブにします。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}/archive`
- パスパラメータ:
  - `id`: TodoのID
- `Content-Type`: `application/json`
- メソッド: `POST`
- リクエストボディ:
  - `archived`: アーカイブする場合は`true`、アーカイブ解除する場合は`false`

リクエストボディの例:

```json
{
    "archived": true
}
```

成功した場合、`200 OK`を返します。
レスポンスボディは、[Todoの取得](#todoの取得)のレスポンスボディと同様です。

### Todoの削除

ログインしているユーザーのTodoを削除します。

- アクセス保護: あり
- エンドポイント: `/todos/{todo_id}`
- パスパラメータ:
  - `id`: TodoのID
- メソッド: `DELETE`

成功した場合、`204 No Content`を返します。
