use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait SqsSender {
    async fn send_message(&self, message: &str) -> Result<()>;
}

pub struct SqsSenderImpl {
    pub client: aws_sdk_sqs::Client,
    pub queue_url: String,
    pub delay_seconds: Option<i32>,
}

#[async_trait]
impl SqsSender for SqsSenderImpl {
    async fn send_message(&self, message: &str) -> Result<()> {
        let _ = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .set_delay_seconds(self.delay_seconds)
            .set_message_body(Some(message.to_string()))
            .send()
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;
    use testresult::TestResult;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn send_test() -> TestResult {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let sqs_sender = SqsSenderImpl {
            client: aws_sdk_sqs::Client::new(&sdk_config),
            queue_url: "opxs-batch-email-send-sqs".to_string(),
            delay_seconds: None,
        };
        sqs_sender.send_message("{}").await?;

        Ok(())
    }
}
