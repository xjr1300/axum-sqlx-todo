use crate::DomainResult;

#[async_trait::async_trait]
pub trait LookupRepository {
    type Entity;
    type Code;

    async fn list(&self) -> DomainResult<Vec<Self::Entity>>;
    async fn by_code(&self, code: &Self::Code) -> DomainResult<Option<Self::Entity>>;
}
