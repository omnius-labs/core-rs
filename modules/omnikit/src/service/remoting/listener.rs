use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv, FramedSend, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion};

#[allow(unused)]
pub struct OmniRemotingListener {
    receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>>,
    sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>>,
    function_id: u32,
}

impl OmniRemotingListener {
    pub async fn new<T>(stream: T, max_frame_length: usize) -> Result<Self>
    where
        T: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>> = Arc::new(TokioMutex::new(Box::new(FramedReceiver::new(reader, max_frame_length))));
        let sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>> = Arc::new(TokioMutex::new(Box::new(FramedSender::new(writer, max_frame_length))));

        let function_id = Self::handshake(receiver.clone()).await?;

        Ok(OmniRemotingListener { sender, receiver, function_id })
    }

    async fn handshake(receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>>) -> Result<u32> {
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
        F: AsyncFnOnce(OmniRemotingStream),
    {
        callback(OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())).await;
        Ok(())
    }
}
