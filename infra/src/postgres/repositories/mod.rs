mod todo;
mod user;
pub use todo::*;
pub use user::*;

use std::marker::PhantomData;

use sqlx::{PgPool, Postgres, Transaction};

use domain::{DomainError, DomainErrorKind, DomainResult};

/// PostgreSQLトランザクション
pub type PgTransaction<'a> = Transaction<'a, Postgres>;

/// PostgreSQLリポジトリ
#[derive(Clone)]
pub struct PgRepository<T> {
    pool: PgPool,
    _marker: PhantomData<T>,
}

impl<T> PgRepository<T> {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            _marker: PhantomData,
        }
    }

    /// トランザクションを開始する。
    ///
    /// # 戻り値
    ///
    /// トランザクション
    pub async fn begin(&self) -> DomainResult<PgTransaction<'_>> {
        self.pool.begin().await.map_err(repository_error)
    }
}

/// トランザクションをコミットする。
///
/// # 引数
///
/// * `tx`: トランザクション
pub async fn commit(tx: PgTransaction<'_>) -> DomainResult<()> {
    tx.commit().await.map_err(repository_error)
}

fn repository_error(e: sqlx::Error) -> DomainError {
    DomainError {
        kind: DomainErrorKind::Repository,
        messages: vec![format!("{e}").into()],
        source: e.into(),
    }
}
