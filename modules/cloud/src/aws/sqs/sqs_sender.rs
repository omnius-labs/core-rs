use async_trait::async_trait;

#[async_trait]
pub trait SqsSender {
    async fn send_message(&self, queue_url: &str, delay_seconds: i32, message_body: &str) -> anyhow::Result<()>;
}

pub struct SqsSenderImpl {
    client: aws_sdk_sqs::Client,
}

impl SqsSenderImpl {
    #[allow(dead_code)]
    pub async fn new(config: aws_config::SdkConfig) -> anyhow::Result<Self> {
        let client = aws_sdk_sqs::Client::new(&config);
        Ok(Self { client })
    }
}

#[async_trait]
impl SqsSender for SqsSenderImpl {
    async fn send_message(&self, queue_url: &str, delay_seconds: i32, message_body: &str) -> anyhow::Result<()> {
        let _output = self
            .client
            .send_message()
            .queue_url(queue_url)
            .delay_seconds(delay_seconds)
            .message_body(message_body)
            .send()
            .await?;

        Ok(())
    }
}
