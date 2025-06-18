pub mod primitives;
mod todo;
mod user;

pub use todo::*;
pub use user::*;

#[macro_export]
macro_rules! sqlx_encode_value {
    ($name:ident, $raw_ty:ty, $oid:literal) => {
        impl sqlx::Encode<'_, sqlx::Postgres> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut sqlx::postgres::PgArgumentBuffer,
            ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
                let code = *self as $raw_ty;
                buf.extend(code.to_be_bytes());
                Ok(sqlx::encode::IsNull::No)
            }
        }

        impl sqlx::Type<sqlx::Postgres> for $name {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                sqlx::postgres::PgTypeInfo::with_oid(sqlx::postgres::types::Oid($oid))
            }
        }
    };
}
