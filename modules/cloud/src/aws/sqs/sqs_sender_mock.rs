use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::SqsSender;

pub struct SqsSenderMock {
    pub send_message_inputs: Arc<Mutex<Vec<SendMessageInput>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMessageInput {
    pub message_body: String,
}

#[async_trait]
impl SqsSender for SqsSenderMock {
    async fn send_message(&self, message_body: &str) -> anyhow::Result<()> {
        self.send_message_inputs.lock().unwrap().push(SendMessageInput {
            message_body: message_body.to_string(),
        });
        Ok(())
    }
}

impl SqsSenderMock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            send_message_inputs: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl Default for SqsSenderMock {
    fn default() -> Self {
        Self::new()
    }
}
