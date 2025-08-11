use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::{EmptyRocketMessage, RocketMessage};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::{CallResult, HelloMessage, OmniRemotingStream, OmniRemotingVersion, PacketMessage};

#[allow(unused)]
pub struct OmniRemotingCaller<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: u32,
    _phantom: std::marker::PhantomData<TErrorMessage>,
}

impl<R, W, TErrorMessage> OmniRemotingCaller<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    pub fn new(reader: R, writer: W, max_frame_length: usize, function_id: u32) -> Self {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        OmniRemotingCaller {
            sender,
            receiver,
            function_id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn handshake(&self) -> Result<()> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id: self.function_id,
        };
        self.sender.lock().await.send(hello_message.export()?).await?;

        Ok(())
    }

    pub async fn call_unary<TParamMessage, TResultMessage>(&self, param: TParamMessage) -> CallResult<TResultMessage, TErrorMessage>
    where
        TParamMessage: RocketMessage + Send + Sync + 'static,
        TResultMessage: RocketMessage + Send + Sync + 'static,
    {
        let param = PacketMessage::<TParamMessage, EmptyRocketMessage>::Completed(param).export()?;
        self.sender.lock().await.send(param).await?;

        let mut message = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<TResultMessage, TErrorMessage>::import(&mut message)?;

        match message {
            PacketMessage::Unknown => Err(Error::new(ErrorKind::UnsupportedType).message("type unknown")),
            PacketMessage::Continue(_) => Err(Error::new(ErrorKind::UnsupportedType).message("type continue")),
            PacketMessage::Completed(result_message) => Ok(Ok(result_message)),
            PacketMessage::Error(error_message) => Ok(Err(error_message)),
        }
    }

    pub fn call_stream<TInputMessage, TOutputMessage>(&self) -> OmniRemotingStream<R, W, TInputMessage, TOutputMessage, TErrorMessage>
    where
        TInputMessage: RocketMessage + Send + Sync + 'static,
        TOutputMessage: RocketMessage + Send + Sync + 'static,
    {
        OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())
    }
}
