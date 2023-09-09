use async_trait::async_trait;

#[async_trait]
pub trait SqsReceiver {
    async fn receive_message(&self, queue_url: &str, wait_time_seconds: i32, max_number_of_messages: i32) -> anyhow::Result<Option<Vec<String>>>;
}

pub struct SqsReceiverImpl {
    client: aws_sdk_sqs::Client,
}

impl SqsReceiverImpl {
    #[allow(dead_code)]
    pub async fn new(config: aws_config::SdkConfig) -> anyhow::Result<Self> {
        let client = aws_sdk_sqs::Client::new(&config);
        Ok(Self { client })
    }
}

#[async_trait]
impl SqsReceiver for SqsReceiverImpl {
    async fn receive_message(&self, queue_url: &str, wait_time_seconds: i32, max_number_of_messages: i32) -> anyhow::Result<Option<Vec<String>>> {
        let output = self
            .client
            .receive_message()
            .queue_url(queue_url)
            .wait_time_seconds(wait_time_seconds)
            .max_number_of_messages(max_number_of_messages)
            .send()
            .await?;

        let res: Option<Vec<String>> = output.messages.map(|n| n.into_iter().flat_map(|m| m.body).collect::<Vec<_>>());
        Ok(res)
    }
}
