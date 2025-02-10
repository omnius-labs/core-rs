use std::{fmt, future::Future, sync::Arc};

use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::RocketMessage;

use crate::service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender};

use super::{HelloMessage, OmniRemotingVersion, PacketMessage, ProtocolErrorCode};

#[allow(unused)]
pub struct OmniRemotingListener<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: Arc<Mutex<Option<u32>>>,
    _phantom: std::marker::PhantomData<TError>,
}

impl<R, W, TError> OmniRemotingListener<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
{
    pub fn new(reader: R, writer: W, max_frame_length: usize) -> Self {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        OmniRemotingListener {
            sender,
            receiver,
            function_id: Arc::new(Mutex::new(None)),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn handshake(&mut self) -> Result<(), super::Error<TError>> {
        let mut v = self
            .receiver
            .lock()
            .await
            .recv()
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::ReceiveFailed))?;
        let hello_message = HelloMessage::import(&mut v).map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::DeserializationFailed))?;

        if hello_message.version == OmniRemotingVersion::V1 {
            *self.function_id.lock() = Some(hello_message.function_id);
            return Ok(());
        }

        Err(super::Error::ProtocolError(super::ProtocolErrorCode::UnsupportedVersion))
    }

    pub fn function_id(&self) -> Result<u32, super::Error<TError>> {
        let v = *self.function_id.lock();
        v.ok_or_else(|| super::Error::ProtocolError(super::ProtocolErrorCode::HandshakeNotFinished))
    }

    pub async fn listen<TParam, TResult, F, Fut>(&self, callback: F) -> Result<(), super::Error<TError>>
    where
        TParam: RocketMessage + Send + Sync + 'static,
        TResult: RocketMessage + Send + Sync + 'static,
        F: FnOnce(TParam) -> Fut,
        Fut: Future<Output = Result<TResult, TError>>,
    {
        let mut param = self
            .receiver
            .lock()
            .await
            .recv()
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::ReceiveFailed))?;
        let param = PacketMessage::<TParam, TError>::import(&mut param)
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::DeserializationFailed))?;

        match param {
            PacketMessage::Unknown => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Continue(_) => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Completed(param) => match callback(param).await {
                Ok(result) => {
                    let result = PacketMessage::<TResult, TError>::Completed(result)
                        .export()
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?;
                    self.sender
                        .lock()
                        .await
                        .send(result)
                        .await
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;
                    Ok(())
                }
                Err(error) => {
                    let error = PacketMessage::<TResult, TError>::Error(error)
                        .export()
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?;
                    self.sender
                        .lock()
                        .await
                        .send(error)
                        .await
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;
                    Ok(())
                }
            },
            PacketMessage::Error(_) => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
        }
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        self.receiver.lock().await.close().await?;
        self.sender.lock().await.close().await?;
        Ok(())
    }
}
