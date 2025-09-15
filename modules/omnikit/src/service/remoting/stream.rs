use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::packet_message::PacketMessage;

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
        OmniRemotingStream { receiver, sender }
    }

    pub async fn send_continue<T>(&self, input_message: T) -> Result<()>
    where
        T: RocketMessage + Send + Sync + 'static,
    {
        let packet = PacketMessage::<T>::Continue(input_message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn send_completed<T>(&self, input_message: T) -> Result<()>
    where
        T: RocketMessage + Send + Sync + 'static,
    {
        let packet = PacketMessage::<T>::Completed(input_message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn recv<T>(&self) -> Result<PacketMessage<T>>
    where
        T: RocketMessage + Send + Sync + 'static,
    {
        let mut bytes = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<T>::import(&mut bytes)?;

        Ok(message)
    }
}
