use async_trait::async_trait;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

use crate::prelude::*;

#[async_trait]
pub trait SesSender {
    async fn send_mail_simple_text(&self, to_address: &str, from_address: &str, subject: &str, body: &str) -> Result<String>;
}

pub struct SesSenderImpl {
    pub client: aws_sdk_sesv2::Client,
    pub configuration_set_name: Option<String>,
}

#[async_trait]
impl SesSender for SesSenderImpl {
    async fn send_mail_simple_text(&self, to_address: &str, from_address: &str, subject: &str, text_body: &str) -> Result<String> {
        let res = self
            .client
            .send_email()
            .destination(Destination::builder().to_addresses(to_address).build())
            .from_email_address(from_address)
            .content(
                EmailContent::builder()
                    .simple(
                        Message::builder()
                            .subject(Content::builder().data(subject).build()?)
                            .body(Body::builder().text(Content::builder().data(text_body).build()?).build())
                            .build(),
                    )
                    .build(),
            )
            .set_configuration_set_name(self.configuration_set_name.clone())
            .send()
            .await?;

        Ok(res.message_id.ok_or_else(|| Error::builder().kind(ErrorKind::NotFound).build())?)
    }
}

#[cfg(test)]
mod tests {
    use aws_config::BehaviorVersion;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn secret_reader_test() {
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let sender = SesSenderImpl {
            client: aws_sdk_sesv2::Client::new(&sdk_config),
            configuration_set_name: None,
        };
        let r = sender
            .send_mail_simple_text("lyrise1984@gmail.com", "no-reply@opxs-dev.omnius-labs.com", "test subject", "test body")
            .await;
        if let Err(e) = r {
            println!("{:?}", e);
        }
    }
}
