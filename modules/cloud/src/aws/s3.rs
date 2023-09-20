use async_trait::async_trait;
use aws_sdk_s3::presigning::PresigningConfig;
use chrono::{DateTime, Duration, Utc};

#[async_trait]
trait S3 {
    async fn gen_get_presigned_uri(&self, bucket: &str, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String>;
    async fn gen_put_presigned_uri(&self, bucket: &str, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String>;
}
pub struct S3Impl {
    pub client: aws_sdk_s3::Client,
}

#[async_trait]
impl S3 for S3Impl {
    async fn gen_get_presigned_uri(&self, bucket: &str, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String> {
        let presigning_config = PresigningConfig::builder()
            .start_time(start_time.into())
            .expires_in(expires_in.to_std()?)
            .build()?;

        let request = self.client.get_object().bucket(bucket).key(key).presigned(presigning_config).await?;
        Ok(request.uri().to_string())
    }

    async fn gen_put_presigned_uri(&self, bucket: &str, key: &str, start_time: DateTime<Utc>, expires_in: Duration) -> anyhow::Result<String> {
        let presigning_config = PresigningConfig::builder()
            .start_time(start_time.into())
            .expires_in(expires_in.to_std()?)
            .build()?;

        let request = self.client.put_object().bucket(bucket).key(key).presigned(presigning_config).await?;
        Ok(request.uri().to_string())
    }
}
