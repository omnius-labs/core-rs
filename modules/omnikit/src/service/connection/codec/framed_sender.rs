use async_trait::async_trait;
use futures_util::SinkExt as _;
use tokio::io::AsyncWrite;
use tokio_util::bytes::Bytes;

use crate::Result;

#[async_trait]
pub trait FramedSend {
    /// Updates the maximum message length accepted by subsequent sends.
    fn set_max_frame_length(&mut self, max_frame_length: usize);

    async fn send(&mut self, buffer: Bytes) -> Result<()>;
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

    /// Updates the maximum message length accepted by subsequent sends.
    pub fn set_max_frame_length(&mut self, max_frame_length: usize) {
        self.framed.encoder_mut().set_max_frame_length(max_frame_length);
    }
}

#[async_trait]
impl<T> FramedSend for FramedSender<T>
where
    T: AsyncWrite + Send + Unpin,
{
    fn set_max_frame_length(&mut self, max_frame_length: usize) {
        FramedSender::set_max_frame_length(self, max_frame_length);
    }

    async fn send(&mut self, buffer: Bytes) -> Result<()> {
        self.framed.send(buffer).await?;
        self.framed.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use testresult::TestResult;
    use tokio::{io::AsyncReadExt as _, sync::Mutex};

    use super::*;

    #[tokio::test]
    async fn raised_limit_allows_rejected_message_through_trait_object() -> TestResult {
        let (writer, mut reader) = tokio::io::duplex(64);
        let sender: Arc<Mutex<dyn FramedSend + Send + Unpin>> = Arc::new(Mutex::new(FramedSender::new(writer, 4)));
        let mut sender = sender.lock().await;
        let message = Bytes::from_static(b"hello");

        assert!(sender.send(message.clone()).await.is_err());
        sender.set_max_frame_length(5);
        sender.send(message).await?;

        let mut received = [0; 9];
        reader.read_exact(&mut received).await?;
        assert_eq!(&received, b"\x05\x00\x00\x00hello");
        assert!(futures_util::poll!(Box::pin(reader.read_u8())).is_pending());

        Ok(())
    }

    #[tokio::test]
    async fn lowered_limit_rejects_message() -> TestResult {
        let mut sender = FramedSender::new(Vec::new(), 5);
        sender.send(Bytes::from_static(b"hello")).await?;

        sender.set_max_frame_length(4);
        assert!(sender.send(Bytes::from_static(b"world")).await.is_err());
        sender.send(Bytes::from_static(b"test")).await?;
        assert_eq!(sender.into_inner(), b"\x05\x00\x00\x00hello\x04\x00\x00\x00test");

        Ok(())
    }
}
