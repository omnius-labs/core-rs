use std::sync::Arc;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedSend as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion};

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
    pub async fn new(reader: R, writer: W, max_frame_length: usize, function_id: u32) -> Result<Self> {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        Self::handshake(sender.clone(), function_id).await?;

        Ok(OmniRemotingCaller { sender, receiver, function_id })
    }

    async fn handshake(sender: Arc<TokioMutex<FramedSender<W>>>, function_id: u32) -> Result<()> {
        let hello_message = HelloMessage {
            version: OmniRemotingVersion::V1,
            function_id,
        };
        sender.lock().await.send(hello_message.export()?.into()).await?;

        Ok(())
    }

    pub fn call_stream(&self) -> OmniRemotingStream<R, W> {
        OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())
    }
}
