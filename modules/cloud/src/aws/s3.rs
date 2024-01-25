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

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn simple_test() {
        env::set_var("AWS_PROFILE", "opxs-dev");
        env::set_var("AWS_REGION", "us-east-1");
        let sdk_config = aws_config::load_from_env().await;
        let s3 = S3Impl {
            client: aws_sdk_s3::Client::new(&sdk_config),
        };
        let uri = s3
            .gen_put_presigned_uri("opxs.v1.dev.test", "test.txt", Utc::now(), Duration::minutes(5))
            .await
            .unwrap();
        println!("{:?}", uri);
        let client = reqwest::Client::new();
        let res = client.put(&uri).body("test").send().await.unwrap();
        println!("{:?}", res);
    }
}
