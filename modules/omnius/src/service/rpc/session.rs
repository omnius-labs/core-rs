use std::sync::Arc;

use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};
use tokio_util::bytes::{Buf as _, Bytes};

use crate::connection::framed::{FramedReceiver, FramedRecv, FramedSend, FramedSender};

use super::error::Error;

#[async_trait]
pub trait OmniRpcSessionFactory {
    async fn create(&self) -> Result<OmniRpcSession, Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OmniRpcSessionType {
    Connected,
    Accepted,
}

#[allow(unused)]
#[derive(Clone)]
pub struct OmniRpcSession {
    id: u32,
    receiver: Arc<TokioMutex<dyn FramedRecv + Send + Unpin>>,
    sender: Arc<TokioMutex<dyn FramedSend + Send + Unpin>>,
}

#[allow(unused)]
impl OmniRpcSession {
    pub async fn new_caller<R, W>(id: u32, reader: R, writer: W, max_frame_length: usize) -> Result<Self, Error>
    where
        R: AsyncRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        let buf = id.to_le_bytes().to_vec();
        sender.as_ref().lock().await.send(Bytes::from(buf)).await?;

        Ok(Self { id, receiver, sender })
    }

    pub async fn new_listener<R, W>(reader: R, writer: W, max_frame_length: usize) -> Result<Self, Error>
    where
        R: AsyncRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        let mut buf = receiver.as_ref().lock().await.recv().await?;
        let id = buf.get_u32_le();

        Ok(Self { id, receiver, sender })
    }
}
