use anyhow::anyhow;
use async_trait::async_trait;

use crate::secrets::secrets_reader::SecretsReader;

pub struct AwsSecretsReader {
    client: aws_sdk_secretsmanager::Client,
}

impl AwsSecretsReader {
    #[allow(dead_code)]
    pub async fn new() -> anyhow::Result<Self> {
        let sdk_config = aws_config::from_env().load().await;
        let client = aws_sdk_secretsmanager::Client::new(&sdk_config);
        Ok(Self { client })
    }
}

#[async_trait]
impl SecretsReader for AwsSecretsReader {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<serde_json::Value> {
        let res = self.client.get_secret_value().secret_id(secret_id).send().await?;

        let json = res.secret_string().ok_or_else(|| anyhow!("not found"))?;
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn secret_reader_test() {
        let secret_reader = AwsSecretsReader::new().await.unwrap();
        let result = secret_reader.read_value("opxs-api").await.unwrap();
        println!("{result}");
    }
}
