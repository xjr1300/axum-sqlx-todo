use domain::{
    DomainResult,
    models::{Role, RoleCode, TodoStatus, TodoStatusCode},
    repositories::LookupRepository,
};

#[async_trait::async_trait]
pub trait LookupUseCase<R>
where
    R: LookupRepository,
    R::Code: Send + Sync,
{
    fn repo(&self) -> &R;

    async fn list(&self) -> DomainResult<Vec<R::Entity>> {
        self.repo().list().await
    }

    async fn by_code(&self, code: &R::Code) -> DomainResult<Option<R::Entity>> {
        self.repo().by_code(code).await
    }
}

pub struct RoleUseCase<R>
where
    R: LookupRepository<Entity = Role, Code = RoleCode>,
{
    pub repo: R,
}

impl<R> LookupUseCase<R> for RoleUseCase<R>
where
    R: LookupRepository<Entity = Role, Code = RoleCode> + Send + Sync,
{
    fn repo(&self) -> &R {
        &self.repo
    }
}

pub struct TodoStatusUseCase<R>
where
    R: LookupRepository<Entity = TodoStatus, Code = TodoStatusCode>,
{
    pub repo: R,
}

impl<R> LookupUseCase<R> for TodoStatusUseCase<R>
where
    R: LookupRepository<Entity = TodoStatus, Code = TodoStatusCode> + Send + Sync,
{
    fn repo(&self) -> &R {
        &self.repo
    }
}
