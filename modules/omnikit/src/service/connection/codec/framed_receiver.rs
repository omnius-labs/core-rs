use async_trait::async_trait;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt as _;
use tokio_util::bytes::Bytes;

use crate::prelude::*;

#[async_trait]
pub trait FramedRecv {
    async fn recv(&mut self) -> Result<Bytes>;
    async fn close(&mut self) -> Result<()>;
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
    async fn recv(&mut self) -> Result<Bytes> {
        let v = self
            .framed
            .next()
            .await
            .ok_or_else(|| Error::builder().kind(ErrorKind::EndOfStream).build())?;
        Ok(v?.freeze())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
