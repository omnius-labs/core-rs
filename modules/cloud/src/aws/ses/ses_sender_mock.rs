use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::SesSender;

pub struct SesSenderMock {
    pub send_mail_simple_text_inputs: Arc<Mutex<Vec<SendMailSimpleTextInput>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMailSimpleTextInput {
    pub to_address: String,
    pub from_address: String,
    pub subject: String,
    pub text_body: String,
}

#[async_trait]
impl SesSender for SesSenderMock {
    async fn send_mail_simple_text(&self, to_address: &str, from_address: &str, subject: &str, text_body: &str) -> anyhow::Result<()> {
        self.send_mail_simple_text_inputs.lock().unwrap().push(SendMailSimpleTextInput {
            to_address: to_address.to_string(),
            from_address: from_address.to_string(),
            subject: subject.to_string(),
            text_body: text_body.to_string(),
        });
        Ok(())
    }
}

impl SesSenderMock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            send_mail_simple_text_inputs: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl Default for SesSenderMock {
    fn default() -> Self {
        Self::new()
    }
}
