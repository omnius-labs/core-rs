use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::EmptyRocketMessage;

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::packet_message::PacketMessage;

pub struct OmniRemotingStream<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    _phantom: std::marker::PhantomData<TErrorMessage>,
}

impl<R, W, TErrorMessage> OmniRemotingStream<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    pub fn new(receiver: Arc<TokioMutex<FramedReceiver<R>>>, sender: Arc<TokioMutex<FramedSender<W>>>) -> Self {
        OmniRemotingStream {
            receiver,
            sender,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn send_continue<TMessage>(&self, message: TMessage) -> Result<()>
    where
        TMessage: RocketMessage + Send + Sync + 'static,
    {
        let packet = PacketMessage::<TMessage, TErrorMessage>::Continue(message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn send_completed<TMessage>(&self, message: TMessage) -> Result<()>
    where
        TMessage: RocketMessage + Send + Sync + 'static,
    {
        let packet = PacketMessage::<TMessage, TErrorMessage>::Completed(message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }
    pub async fn send_error(&self, error_message: TErrorMessage) -> Result<()> {
        let packet = PacketMessage::<EmptyRocketMessage, TErrorMessage>::Error(error_message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn recv<TMessage>(&self) -> Result<PacketMessage<TMessage, TErrorMessage>>
    where
        TMessage: RocketMessage + Send + Sync + 'static,
    {
        let mut bytes = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<TMessage, TErrorMessage>::import(&mut bytes)?;

        Ok(message)
    }
}
