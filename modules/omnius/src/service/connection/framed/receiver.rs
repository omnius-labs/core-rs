use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::bytes::Bytes;

use super::packet::Packet;

#[async_trait]
pub trait FramedRecv {
    async fn recv(&mut self) -> anyhow::Result<Bytes>;
}

#[async_trait]
pub trait FramedRecvExt: FramedRecv {
    async fn recv_message<T: DeserializeOwned>(&mut self) -> anyhow::Result<T>;
}

pub struct FramedReceiver<T>
where
    T: AsyncRead + Unpin,
{
    framed: tokio_util::codec::FramedRead<T, tokio_util::codec::LengthDelimitedCodec>,
}

#[allow(unused)]
impl<T> FramedReceiver<T>
where
    T: AsyncRead + Unpin,
{
    pub fn new(stream: T, max_frame_length: usize) -> Self {
        let codec = tokio_util::codec::LengthDelimitedCodec::builder()
            .max_frame_length(max_frame_length)
            .little_endian()
            .new_codec();
        let framed = tokio_util::codec::FramedRead::new(stream, codec);
        Self { framed }
    }

    pub fn into_inner(self) -> T {
        self.framed.into_inner()
    }
}

#[async_trait]
impl<T> FramedRecv for FramedReceiver<T>
where
    T: AsyncRead + Send + Unpin,
{
    async fn recv(&mut self) -> anyhow::Result<Bytes> {
        let buffer = self.framed.next().await.ok_or(anyhow::anyhow!("Stream ended"))??.freeze();
        Ok(buffer)
    }
}

#[async_trait]
impl<T: FramedRecv> FramedRecvExt for T
where
    T: ?Sized + Send + Unpin,
{
    async fn recv_message<TItem: DeserializeOwned>(&mut self) -> anyhow::Result<TItem> {
        let b = self.recv().await?;
        let item = Packet::deserialize(b)?;
        Ok(item)
    }
}
