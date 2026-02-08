use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion};

#[allow(unused)]
pub struct OmniRemotingListener<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<ReadHalf<T>>>>,
    sender: Arc<TokioMutex<FramedSender<WriteHalf<T>>>>,
    function_id: u32,
}

impl<T> OmniRemotingListener<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    pub async fn new(stream: T, max_frame_length: usize) -> Result<Self> {
        let (reader, writer) = tokio::io::split(stream);
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        let function_id = Self::handshake(receiver.clone()).await?;

        Ok(OmniRemotingListener { sender, receiver, function_id })
    }

    async fn handshake(receiver: Arc<TokioMutex<FramedReceiver<ReadHalf<T>>>>) -> Result<u32> {
        let v = receiver.lock().await.recv().await?;
        let hello_message = HelloMessage::import(&v)?;

        if hello_message.version == OmniRemotingVersion::V1 {
            return Ok(hello_message.function_id);
        }

        Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported version: {}", hello_message.version)))
    }

    pub fn function_id(&self) -> u32 {
        self.function_id
    }

    pub async fn listen_stream<F>(&self, callback: F) -> Result<()>
    where
        F: AsyncFnOnce(OmniRemotingStream<FramedReceiver<ReadHalf<T>>, FramedSender<WriteHalf<T>>>),
    {
        callback(OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())).await;
        Ok(())
    }
}
