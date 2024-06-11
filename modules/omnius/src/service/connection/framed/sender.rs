use anyhow::Context as _;
use async_trait::async_trait;
use futures_util::SinkExt;
use serde::Serialize;
use tokio::io::AsyncWrite;
use tokio_util::bytes::Bytes;

use super::packet::Packet;

#[async_trait]
pub trait FramedSend {
    async fn send(&mut self, buffer: Bytes) -> anyhow::Result<()>;
}

#[async_trait]
pub trait FramedSendExt: FramedSend {
    async fn send_message<T: Serialize + Send>(&mut self, item: T) -> anyhow::Result<()>;
}

pub struct FramedSender<T>
where
    T: AsyncWrite + Unpin,
{
    framed: tokio_util::codec::FramedWrite<T, tokio_util::codec::LengthDelimitedCodec>,
}

#[allow(unused)]
impl<T> FramedSender<T>
where
    T: AsyncWrite + Unpin,
{
    pub fn new(stream: T, max_frame_length: usize) -> Self {
        let codec = tokio_util::codec::LengthDelimitedCodec::builder()
            .max_frame_length(max_frame_length)
            .little_endian()
            .new_codec();
        let framed = tokio_util::codec::FramedWrite::new(stream, codec);
        Self { framed }
    }

    pub fn into_inner(self) -> T {
        self.framed.into_inner()
    }
}

#[async_trait]
impl<T> FramedSend for FramedSender<T>
where
    T: AsyncWrite + Send + Unpin,
{
    async fn send(&mut self, buffer: Bytes) -> anyhow::Result<()> {
        self.framed.send(buffer).await.with_context(|| "Failed to send")?;
        Ok(())
    }
}

#[async_trait]
impl<T: FramedSend> FramedSendExt for T
where
    T: ?Sized + Send + Unpin,
{
    async fn send_message<TItem: Serialize + Send>(&mut self, item: TItem) -> anyhow::Result<()> {
        let b = Packet::serialize(item)?;
        self.send(b).await?;
        Ok(())
    }
}
