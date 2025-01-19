use async_trait::async_trait;

#[async_trait]
pub trait Terminable {
    type Result;
    async fn terminate(&self) -> Self::Result;
}
