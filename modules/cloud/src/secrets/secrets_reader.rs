use async_trait::async_trait;

#[async_trait]
pub trait SecretsReader {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<serde_json::Value>;
}
