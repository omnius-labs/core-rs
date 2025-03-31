use std::path::Path;

use async_trait::async_trait;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use chrono::{DateTime, Duration, Utc};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::Result;

#[async_trait]
pub trait S3Client {
    async fn gen_get_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration, file_name: &str) -> Result<String>;
    async fn gen_put_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> Result<String>;
    async fn get_object(&self, key: &str, destination: &Path) -> Result<()>;
    async fn put_object(&self, key: &str, source: &Path) -> Result<()>;
}
pub struct S3ClientImpl {
    pub client: aws_sdk_s3::Client,
    pub bucket: String,
}

#[async_trait]
impl S3Client for S3ClientImpl {
    async fn gen_get_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration, file_name: &str) -> Result<String> {
        let presigning_config = PresigningConfig::builder()
            .start_time(start_time.into())
            .expires_in(expires_in.to_std()?)
            .build()?;

        let encoded_file_name = urlencoding::encode(file_name).to_string();

        let request = self
            .client
            .get_object()
            .bucket(self.bucket.as_str())
            .key(key)
            .set_response_content_disposition(Some(format!(
                "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                encoded_file_name, encoded_file_name
            )))
            .presigned(presigning_config)
            .await?;
        Ok(request.uri().to_string())
    }

    async fn gen_put_presigned_uri(&self, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> Result<String> {
        let presigning_config = PresigningConfig::builder()
            .start_time(start_time.into())
            .expires_in(expires_in.to_std()?)
            .build()?;

        let request = self
            .client
            .put_object()
            .bucket(self.bucket.as_str())
            .key(key)
            .presigned(presigning_config)
            .await?;
        Ok(request.uri().to_string())
    }

    async fn get_object(&self, key: &str, destination: &Path) -> Result<()> {
        let mut file = File::create(destination).await?;

        let mut object = self.client.get_object().bucket(self.bucket.as_str()).key(key).send().await?;

        while let Some(bytes) = object.body.try_next().await? {
            file.write_all(&bytes).await?;
        }

        Ok(())
    }

    async fn put_object(&self, key: &str, source: &Path) -> Result<()> {
        let body = ByteStream::from_path(source).await?;
        self.client.put_object().bucket(self.bucket.as_str()).key(key).body(body).send().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use aws_config::BehaviorVersion;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn simple_test() {
        unsafe {
            env::set_var("AWS_PROFILE", "opxs-dev");
            env::set_var("AWS_REGION", "us-east-1");
        }

        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let s3 = S3ClientImpl {
            client: aws_sdk_s3::Client::new(&sdk_config),
            bucket: "opxs.v1.dev.file-convert".to_string(),
        };
        let uri = s3.gen_put_presigned_uri("in/test.txt", Utc::now(), Duration::minutes(5)).await.unwrap();
        println!("{:?}", uri);
        let client = reqwest::Client::new();
        let res = client.put(&uri).body("test").send().await.unwrap();
        println!("{:?}", res);
    }
}
