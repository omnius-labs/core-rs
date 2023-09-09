use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
pub trait SecretsReader {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<String>;
}

pub struct SecretsReaderImpl {
    client: aws_sdk_secretsmanager::Client,
}

impl SecretsReaderImpl {
    #[allow(dead_code)]
    pub async fn new(config: aws_config::SdkConfig) -> anyhow::Result<Self> {
        let client = aws_sdk_secretsmanager::Client::new(&config);
        Ok(Self { client })
    }
}

#[async_trait]
impl SecretsReader for SecretsReaderImpl {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<String> {
        let output = self.client.get_secret_value().secret_id(secret_id).send().await?;

        let res = output.secret_string().ok_or_else(|| anyhow!("not found"))?;
        Ok(res.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn secret_reader_test() {
        let sdk_config = aws_config::from_env().load().await;
        let secret_reader = SecretsReaderImpl::new(sdk_config).await.unwrap();
        let result = secret_reader.read_value("opxs-api").await.unwrap();
        println!("{result}");
    }
}
