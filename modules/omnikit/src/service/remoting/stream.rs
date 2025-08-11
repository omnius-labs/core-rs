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

pub struct OmniRemotingStream<R, W, TInputMessage, TOutputMessage, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TInputMessage: RocketMessage + Send + Sync + 'static,
    TOutputMessage: RocketMessage + Send + Sync + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    _phantom_input: std::marker::PhantomData<TInputMessage>,
    _phantom_output: std::marker::PhantomData<TOutputMessage>,
    _phantom_error: std::marker::PhantomData<TErrorMessage>,
}

impl<R, W, TInputMessage, TOutputMessage, TErrorMessage> OmniRemotingStream<R, W, TInputMessage, TOutputMessage, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TInputMessage: RocketMessage + Send + Sync + 'static,
    TOutputMessage: RocketMessage + Send + Sync + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    pub fn new(receiver: Arc<TokioMutex<FramedReceiver<R>>>, sender: Arc<TokioMutex<FramedSender<W>>>) -> Self {
        OmniRemotingStream {
            receiver,
            sender,
            _phantom_input: std::marker::PhantomData,
            _phantom_output: std::marker::PhantomData,
            _phantom_error: std::marker::PhantomData,
        }
    }

    pub async fn send_continue(&self, input_message: TInputMessage) -> Result<()> {
        let packet = PacketMessage::<TInputMessage, TErrorMessage>::Continue(input_message);

        let bytes = packet.export()?;
        self.sender.lock().await.send(bytes).await?;

        Ok(())
    }

    pub async fn send_completed(&self, input_message: TInputMessage) -> Result<()> {
        let packet = PacketMessage::<TInputMessage, TErrorMessage>::Completed(input_message);

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

    pub async fn recv(&self) -> Result<PacketMessage<TOutputMessage, TErrorMessage>> {
        let mut bytes = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<TOutputMessage, TErrorMessage>::import(&mut bytes)?;

        Ok(message)
    }
}
