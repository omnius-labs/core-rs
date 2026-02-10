use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::{
    prelude::*,
    service::connection::codec::{FramedRecv, FramedSend},
};

pub struct OmniRemotingStream {
    receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>>,
    sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>>,
}

impl OmniRemotingStream {
    pub(crate) fn new(receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>>, sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>>) -> Self {
        Self { receiver, sender }
    }

    pub async fn send<T>(&self, message: T) -> Result<()>
    where
        T: RocketPackStruct + Send + Sync + 'static,
    {
        let bytes = message.export()?;
        self.sender.lock().await.send(bytes.into()).await?;

        Ok(())
    }

    pub async fn recv<T>(&self) -> Result<T>
    where
        T: RocketPackStruct + Send + Sync + 'static,
    {
        let bytes = self.receiver.lock().await.recv().await?;
        let message = T::import(&bytes)?;

        Ok(message)
    }
}
