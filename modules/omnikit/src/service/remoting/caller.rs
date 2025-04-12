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

use super::{CallResult, HelloMessage, OmniRemotingVersion, PacketMessage};

#[allow(unused)]
pub struct OmniRemotingCaller<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: u32,
    _phantom: std::marker::PhantomData<TError>,
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

    pub async fn call<TParamMessage, TSuccessMessage>(&self, param: TParamMessage) -> CallResult<TSuccessMessage, TErrorMessage>
    where
        TParamMessage: RocketMessage + Send + Sync + 'static,
        TSuccessMessage: RocketMessage + Send + Sync + 'static,
    {
        let param = PacketMessage::<TParamMessage, EmptyRocketMessage>::Completed(param).export()?;
        self.sender.lock().await.send(param).await?;

        let mut message = self.receiver.lock().await.recv().await?;
        let message = PacketMessage::<TSuccessMessage, TErrorMessage>::import(&mut message)?;

        match message {
            PacketMessage::Unknown => Err(Error::new(ErrorKind::UnsupportedType).message("type unknown")),
            PacketMessage::Continue(_) => Err(Error::new(ErrorKind::UnsupportedType).message("type continue")),
            PacketMessage::Completed(success) => Ok(Ok(success)),
            PacketMessage::Error(error) => Ok(Err(error)),
        }
    }

    pub async fn close(&self) -> Result<()> {
        self.receiver.lock().await.close().await?;
        self.sender.lock().await.close().await?;
        Ok(())
    }
}
