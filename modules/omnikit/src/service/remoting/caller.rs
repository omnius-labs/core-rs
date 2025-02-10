use std::{fmt, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::{EmptyRocketMessage, RocketMessage};

use crate::service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender};

use super::{HelloMessage, OmniRemotingVersion, PacketMessage, ProtocolErrorCode};

#[allow(unused)]
pub struct OmniRemotingCaller<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: u32,
    _phantom: std::marker::PhantomData<TError>,
}

impl<R, W, TError> OmniRemotingCaller<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
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

    pub async fn handshake(&self) -> Result<(), super::Error<TError>> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id: self.function_id,
        };
        self.sender
            .lock()
            .await
            .send(
                hello_message
                    .export()
                    .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?,
            )
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;

        Ok(())
    }

    pub async fn call<TParam, TResult>(&self, param: TParam) -> Result<TResult, super::Error<TError>>
    where
        TParam: RocketMessage + Send + Sync + 'static,
        TResult: RocketMessage + Send + Sync + 'static,
    {
        let param = PacketMessage::<TParam, EmptyRocketMessage>::Completed(param)
            .export()
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?;
        self.sender
            .lock()
            .await
            .send(param)
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;

        let mut result = self
            .receiver
            .lock()
            .await
            .recv()
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::ReceiveFailed))?;
        let result = PacketMessage::<TResult, TError>::import(&mut result)
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::DeserializationFailed))?;

        match result {
            PacketMessage::Unknown => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Continue(_) => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Completed(result) => Ok(result),
            PacketMessage::Error(error) => Err(super::Error::ApplicationError(error)),
        }
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        self.receiver.lock().await.close().await?;
        self.sender.lock().await.close().await?;
        Ok(())
    }
}
