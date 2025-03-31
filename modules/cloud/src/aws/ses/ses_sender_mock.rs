use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::Result;

use super::SesSender;

pub struct SesSenderMock {
    pub send_mail_simple_text_inputs: Arc<Mutex<Vec<SendMailSimpleTextInput>>>,
    pub send_mail_simple_text_outputs: Arc<Mutex<VecDeque<String>>>,
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
    async fn send_mail_simple_text(&self, to_address: &str, from_address: &str, subject: &str, text_body: &str) -> Result<String> {
        self.send_mail_simple_text_inputs.lock().push(SendMailSimpleTextInput {
            to_address: to_address.to_string(),
            from_address: from_address.to_string(),
            subject: subject.to_string(),
            text_body: text_body.to_string(),
        });

        let output = self.send_mail_simple_text_outputs.lock().pop_front().unwrap_or_default();
        Ok(output)
    }
}

impl SesSenderMock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            send_mail_simple_text_inputs: Arc::new(Mutex::new(vec![])),
            send_mail_simple_text_outputs: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl Default for SesSenderMock {
    fn default() -> Self {
        Self::new()
    }
}
