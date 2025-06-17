use enum_display::EnumDisplay;
use garde::Validate as _;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::OffsetDateTime;

use utils::serde::{deserialize_option_offset_datetime, serialize_option_offset_datetime};

use super::primitives::Id;
use crate::{
    DomainError, DomainErrorKind, DomainResult, domain_error, impl_string_primitive,
    models::primitives::{Description, DisplayOrder},
};

/// ユーザーID
pub type UserId = Id<User>;

/// ユーザーの苗字
#[derive(Debug, Clone, garde::Validate)]
pub struct FamilyName(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(FamilyName);

/// ユーザーの名前
#[derive(Debug, Clone, garde::Validate)]
pub struct GivenName(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(GivenName);

/// Eメールアドレス
#[derive(Debug, Clone, garde::Validate)]
pub struct Email(#[garde(email)] pub String);
impl_string_primitive!(Email);

/// PHC文字列
///
/// PHC(Password Hashing Competition)文字列は、パスワードのハッシュを表現するための標準形式である。
/// PHC文字列は次の形式で、`$`で区切られた次のパーツで構成される。
///
/// ```text
/// $argon2id$v=19$m=65536,t=2,p=1$Y2F0$E2r1h/6vl6eEuNTmdNG49w
/// ```
///
/// - `$argon2id$` - 使用するハッシュアルゴリズム
/// - `v=19` - バージョン番号
/// - `m=65536,t=2,p=1` - ハッシュのパラメータ（メモリ=64MiB, タイムコスト=2, 並列度=1）
/// - `Y2F0` - ソルト（Base64エンコードされた値）
/// - `E2r1h/6vl6eEuNTmdNG49w` - ハッシュ値（Base64エンコードされた値）
#[derive(Debug, Clone)]
pub struct PHCString(pub SecretString);

impl PHCString {
    pub fn new(value: SecretString) -> DomainResult<Self> {
        let value = value.expose_secret();
        if value.is_empty() || value.len() > 255 {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "The length of PHC strings should be less or equal to 255 characters",
            ));
        }
        Ok(Self(SecretString::new(value.into())))
    }
}

/// アクセストークン
#[derive(Debug, Clone)]
pub struct AccessToken(pub SecretString);

/// リフレッシュトークン
#[derive(Debug, Clone)]
pub struct RefreshToken(pub SecretString);

/// ユーザー
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// ID
    pub id: UserId,
    /// 苗字
    pub family_name: FamilyName,
    /// 名前
    pub given_name: GivenName,
    /// Eメールアドレス
    pub email: Email,
    /// ロール
    pub role: Role,
    /// アクティブフラグ
    pub active: bool,
    /// 最終ログイン日時
    #[serde(serialize_with = "serialize_option_offset_datetime")]
    #[serde(deserialize_with = "deserialize_option_offset_datetime")]
    pub last_login_at: Option<OffsetDateTime>,
    /// 作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// 更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// ログイン失敗履歴
///
/// 連続ログイン試行許容時間内に、ログインに失敗した回数を記録する。
#[derive(Debug, Clone, Copy)]
pub struct LoginFailedHistory {
    /// ユーザーID
    pub user_id: UserId,
    /// 最初に試行に失敗した日時
    pub attempted_at: OffsetDateTime,
    /// 試行回数
    pub number_of_attempts: u32,
    /// 作成日時
    pub created_at: OffsetDateTime,
    /// 更新日時
    pub updated_at: OffsetDateTime,
}

/// ロールコード
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumDisplay, Serialize_repr, Deserialize_repr,
)]
#[enum_display(case = "Snake")]
#[repr(i16)]
pub enum RoleCode {
    Admin = 1,
    User = 2,
}

impl TryFrom<i16> for RoleCode {
    type Error = DomainError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RoleCode::Admin),
            2 => Ok(RoleCode::User),
            _ => Err(domain_error(
                DomainErrorKind::Validation,
                "Invalid role code",
            )),
        }
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for RoleCode {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let code = *self as i16;
        buf.extend(code.to_be_bytes());
        Ok(sqlx::encode::IsNull::No)
    }
}

impl sqlx::Type<sqlx::Postgres> for RoleCode {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        // OID 21 is the OID for `int2` in PostgreSQL, which corresponds to i16
        sqlx::postgres::PgTypeInfo::with_oid(sqlx::postgres::types::Oid(21))
    }
}

/// ロール名
#[derive(Debug, Clone, garde::Validate)]
pub struct RoleName(#[garde(length(chars, min = 1, max = 50))] pub String);
impl_string_primitive!(RoleName);

/// ロール説明
#[derive(Debug, Clone, garde::Validate)]
pub struct RoleDescription(#[garde(length(chars, min = 1, max = 255))] pub String);
impl_string_primitive!(RoleDescription);

/// ロール
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    /// コード
    pub code: RoleCode,
    /// 名称
    pub name: RoleName,
    /// 説明
    pub description: Option<Description>,
    /// 表示順
    pub display_order: DisplayOrder,
    /// 作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// 更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
