use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::RocketMessage;

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion, PacketMessage};

#[allow(unused)]
pub struct OmniRemotingCaller<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: u32,
}

impl<R, W> OmniRemotingCaller<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub fn new(reader: R, writer: W, max_frame_length: usize, function_id: u32) -> Self {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        OmniRemotingCaller { sender, receiver, function_id }
    }

    pub async fn handshake(&self) -> Result<()> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id: self.function_id,
        };
        self.sender.lock().await.send(hello_message.export()?).await?;

        Ok(())
    }

    pub async fn call_unary<TParamMessage, TResultMessage>(&self, param: TParamMessage) -> Result<TResultMessage>
    where
        TParamMessage: RocketMessage + Send + Sync + 'static,
        TResultMessage: RocketMessage + Send + Sync + 'static,
    {
        let param = PacketMessage::<TParamMessage>::Completed(param).export()?;
        self.sender.lock().await.send(param).await?;

        let mut message = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<TResultMessage>::import(&mut message)?;

        match message {
            PacketMessage::Unknown => Err(Error::builder().kind(ErrorKind::UnsupportedType).message("type unknown").build()),
            PacketMessage::Continue(_) => Err(Error::builder().kind(ErrorKind::UnsupportedType).message("type continue").build()),
            PacketMessage::Completed(result_message) => Ok(result_message),
        }
    }

    pub fn call_stream(&self) -> OmniRemotingStream<R, W> {
        OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())
    }
}
