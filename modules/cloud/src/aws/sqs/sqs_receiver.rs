use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait SqsReceiver {
    async fn receive_message(&self) -> Result<Option<Vec<String>>>;
}

pub struct SqsReceiverImpl {
    pub client: aws_sdk_sqs::Client,
    pub queue_url: String,
    pub wait_time_seconds: Option<i32>,
    pub max_number_of_messages: Option<i32>,
}

#[async_trait]
impl SqsReceiver for SqsReceiverImpl {
    async fn receive_message(&self) -> Result<Option<Vec<String>>> {
        let output = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .set_wait_time_seconds(self.wait_time_seconds)
            .set_max_number_of_messages(self.max_number_of_messages)
            .send()
            .await?;

        let res: Option<Vec<String>> = output.messages.map(|n| n.into_iter().flat_map(|m| m.body).collect::<Vec<_>>());
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;
    use testresult::TestResult;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn receive_test() -> TestResult {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let secret_reader = SqsReceiverImpl {
            client: aws_sdk_sqs::Client::new(&sdk_config),
            queue_url: "opxs-batch-email-send-sqs".to_string(),
            wait_time_seconds: None,
            max_number_of_messages: None,
        };
        let result = secret_reader.receive_message().await?;
        println!("{result:?}");

        Ok(())
    }
}
