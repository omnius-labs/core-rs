use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::RocketMessage;

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion};

#[allow(unused)]
pub struct OmniRemotingListener<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: u32,
}

impl<R, W> OmniRemotingListener<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub async fn new(reader: R, writer: W, max_frame_length: usize) -> Result<Self> {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        let function_id = Self::handshake(receiver.clone()).await?;

        Ok(OmniRemotingListener { sender, receiver, function_id })
    }

    async fn handshake(receiver: Arc<TokioMutex<FramedReceiver<R>>>) -> Result<u32> {
        let mut v = receiver.lock().await.recv().await?;
        let hello_message = HelloMessage::import(&mut v)?;

        if hello_message.version == OmniRemotingVersion::V1 {
            return Ok(hello_message.function_id);
        }

        Err(Error::builder()
            .kind(ErrorKind::UnsupportedType)
            .message(format!("unsupported version: {}", hello_message.version))
            .build())
    }

    pub fn function_id(&self) -> u32 {
        self.function_id
    }

    pub async fn listen_stream<F>(&self, callback: F) -> Result<()>
    where
        F: AsyncFnOnce(OmniRemotingStream<R, W>),
    {
        callback(OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())).await;
        Ok(())
    }
}
