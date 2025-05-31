mod todo;
mod user;

pub use todo::*;
pub use user::*;

use std::marker::PhantomData;

use sqlx::{PgPool, Postgres, Transaction};

use domain::{DomainError, DomainResult};

/// PostgreSQLトランザクション
pub type PgTransaction<'a> = Transaction<'a, Postgres>;

/// PostgreSQLリポジトリ
pub struct PgRepository<T> {
    pub pool: PgPool,
    pub _marker: PhantomData<T>,
}

impl<T> PgRepository<T> {
    /// トランザクションを開始する。
    ///
    /// # 戻り値
    ///
    /// トランザクション
    pub async fn begin(&self) -> DomainResult<PgTransaction<'_>> {
        self.pool
            .begin()
            .await
            .map_err(|e| DomainError::Repository(e.to_string().into()))
    }
}

/// トランザクションをコミットする。
///
/// # 引数
///
/// * `tx`: トランザクション
pub async fn commit(tx: PgTransaction<'_>) -> DomainResult<()> {
    tx.commit()
        .await
        .map_err(|e| DomainError::Repository(e.to_string().into()))
}
