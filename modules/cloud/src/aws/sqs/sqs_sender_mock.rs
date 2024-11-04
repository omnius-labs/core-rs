use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::SqsSender;

pub struct SqsSenderMock {
    pub send_message_inputs: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl SqsSender for SqsSenderMock {
    async fn send_message(&self, message: &str) -> anyhow::Result<()> {
        self.send_message_inputs.lock().push(message.to_string());
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
