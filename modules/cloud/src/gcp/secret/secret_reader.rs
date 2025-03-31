use async_trait::async_trait;
use gcloud_sdk::{
    GoogleApi, GoogleAuthMiddleware,
    google::cloud::secretmanager::v1::{AccessSecretVersionRequest, secret_manager_service_client::SecretManagerServiceClient},
};

use crate::{Error, ErrorKind, Result};

#[async_trait]
pub trait SecretReader {
    async fn read_value(&self, secret_id: &str) -> Result<String>;
}

pub struct SecretReaderImpl {}

#[async_trait]
impl SecretReader for SecretReaderImpl {
    async fn read_value(&self, secret_id: &str) -> Result<String> {
        let client: GoogleApi<SecretManagerServiceClient<GoogleAuthMiddleware>> =
            GoogleApi::from_function(SecretManagerServiceClient::new, "https://secretmanager.googleapis.com", None).await?;

        let request = AccessSecretVersionRequest { name: secret_id.to_string() };

        let response = client.get().access_secret_version(request).await?;

        let result = response
            .get_ref()
            .payload
            .as_ref()
            .map(|p| p.data.as_sensitive_str())
            .ok_or_else(|| Error::new(ErrorKind::NotFound))?;

        Ok(result.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn secrets_reader_test() {
        let secret_reader = SecretReaderImpl {};
        let result = secret_reader
            .read_value("projects/bews-415522/secrets/bews-secret/versions/latest")
            .await
            .unwrap();
        println!("{result}");
    }
}
