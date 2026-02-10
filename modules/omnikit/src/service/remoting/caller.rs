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
pub struct OmniRemotingCaller {
    receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>>,
    sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>>,
    function_id: u32,
}

impl OmniRemotingCaller {
    pub async fn new<T>(stream: T, max_frame_length: usize, function_id: u32) -> Result<Self>
    where
        T: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let receiver: Arc<TokioMutex<Box<dyn FramedRecv + Send>>> = Arc::new(TokioMutex::new(Box::new(FramedReceiver::new(reader, max_frame_length))));
        let sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>> = Arc::new(TokioMutex::new(Box::new(FramedSender::new(writer, max_frame_length))));

        Self::handshake(sender.clone(), function_id).await?;

        Ok(OmniRemotingCaller { sender, receiver, function_id })
    }

    async fn handshake(sender: Arc<TokioMutex<Box<dyn FramedSend + Send>>>, function_id: u32) -> Result<()> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id,
        };
        sender.lock().await.send(hello_message.export()?.into()).await?;

        Ok(())
    }

    pub fn call_stream(&self) -> OmniRemotingStream {
        OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())
    }
}
