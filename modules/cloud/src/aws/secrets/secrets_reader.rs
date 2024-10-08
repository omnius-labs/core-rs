use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
pub trait SecretsReader {
    async fn read_value(&self, secret_id: &str) -> anyhow::Result<String>;
}

pub struct SecretsReaderImpl {
    pub client: aws_sdk_secretsmanager::Client,
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
    use aws_config::BehaviorVersion;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn secrets_reader_test() {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let secret_reader = SecretsReaderImpl {
            client: aws_sdk_secretsmanager::Client::new(&sdk_config),
        };
        let result = secret_reader.read_value("opxs-api").await.unwrap();
        println!("{result}");
    }
}
