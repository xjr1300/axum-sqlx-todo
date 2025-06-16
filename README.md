# rusty-todo

- [rusty-todo](#rusty-todo)
  - [ユーザーエンドポイント](#ユーザーエンドポイント)
    - [ユーザー登録](#ユーザー登録)
      - [パスワード](#パスワード)
    - [ログイン](#ログイン)
      - [保護されたAPIへのリクエスト](#保護されたapiへのリクエスト)
    - [ユーザー情報取得](#ユーザー情報取得)
    - [ユーザー情報の更新](#ユーザー情報の更新)
    - [トークンのリフレッシュ](#トークンのリフレッシュ)
    - [ログアウト](#ログアウト)

Todoを管理するWeb APIサービスです。

APIは、RESTfulな設計にあまり**従っていません**。

また、クエリサービスの機能をリポジトリが担っているため、ドメイン駆動設計に従っていません。
さらに、データを更新するエンドポイントが、データを帰す場合があるなど、コマンドクエリ分離の原則に**厳密に**従っていません。

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
