use async_trait::async_trait;

#[async_trait]
pub trait Terminable {
    type Error;
    async fn terminate(&self) -> Result<(), Self::Error>;
}
