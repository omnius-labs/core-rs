use async_trait::async_trait;
use futures_util::SinkExt as _;
use tokio::io::AsyncWrite;
use tokio_util::bytes::Bytes;

use crate::Result;

#[async_trait]
pub trait FramedSend {
    async fn send(&mut self, buffer: Bytes) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
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
    async fn send(&mut self, buffer: Bytes) -> Result<()> {
        self.framed.send(buffer).await?;
        self.framed.flush().await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.framed.flush().await?;
        self.framed.close().await?;
        Ok(())
    }
}
