use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedSend as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion};

#[allow(unused)]
pub struct OmniRemotingCaller<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<ReadHalf<T>>>>,
    sender: Arc<TokioMutex<FramedSender<WriteHalf<T>>>>,
    function_id: u32,
}

impl<T> OmniRemotingCaller<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    pub async fn new(stream: T, max_frame_length: usize, function_id: u32) -> Result<Self> {
        let (reader, writer) = tokio::io::split(stream);
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        Self::handshake(sender.clone(), function_id).await?;

        Ok(OmniRemotingCaller { sender, receiver, function_id })
    }

    async fn handshake(sender: Arc<TokioMutex<FramedSender<WriteHalf<T>>>>, function_id: u32) -> Result<()> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id,
        };
        sender.lock().await.send(hello_message.export()?.into()).await?;

        Ok(())
    }

    pub fn call_stream(&self) -> OmniRemotingStream<FramedReceiver<ReadHalf<T>>, FramedSender<WriteHalf<T>>> {
        OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())
    }
}
