use async_trait::async_trait;

#[async_trait]
pub trait SecretReader {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<serde_json::Value>;
}
