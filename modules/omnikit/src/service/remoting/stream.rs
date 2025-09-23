use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

pub struct OmniRemotingStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
}

impl<R, W> OmniRemotingStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub fn new(receiver: Arc<TokioMutex<FramedReceiver<R>>>, sender: Arc<TokioMutex<FramedSender<W>>>) -> Self {
        Self { receiver, sender }
    }

    pub async fn send<T>(&self, message: T) -> Result<()>
    where
        T: RocketMessage + Send + Sync + 'static,
    {
        let bytes = message.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn recv<T>(&self) -> Result<T>
    where
        T: RocketMessage + Send + Sync + 'static,
    {
        let mut bytes = self.receiver.lock().await.recv().await?;
        let message = T::import(&mut bytes)?;

        Ok(message)
    }
}
