use async_trait::async_trait;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt as _;
use tokio_util::bytes::Bytes;

use crate::prelude::*;

#[async_trait]
pub trait FramedRecv {
    /// Updates the limit checked when decoding a frame header, including headers already buffered.
    /// A frame whose header has already been decoded keeps its previously accepted length.
    fn set_max_frame_length(&mut self, max_frame_length: usize);

    async fn recv(&mut self) -> Result<Bytes>;
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

    /// Updates the limit checked when decoding a frame header, including headers already buffered.
    /// A frame whose header has already been decoded keeps its previously accepted length.
    pub fn set_max_frame_length(&mut self, max_frame_length: usize) {
        self.framed.decoder_mut().set_max_frame_length(max_frame_length);
    }
}

#[async_trait]
impl<T> FramedRecv for FramedReceiver<T>
where
    T: AsyncRead + Send + Unpin,
{
    fn set_max_frame_length(&mut self, max_frame_length: usize) {
        FramedReceiver::set_max_frame_length(self, max_frame_length);
    }

    async fn recv(&mut self) -> Result<Bytes> {
        let v = self.framed.next().await.ok_or_else(|| Error::new(ErrorKind::EndOfStream))?;
        Ok(v?.freeze())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use testresult::TestResult;
    use tokio::{io::AsyncWriteExt as _, sync::Mutex};

    use super::*;

    #[tokio::test]
    async fn rejects_frame_over_limit() {
        let mut receiver = FramedReceiver::new(&b"\x05\x00\x00\x00hello"[..], 4);

        assert!(receiver.recv().await.is_err());
    }

    #[tokio::test]
    async fn raised_limit_accepts_buffered_frame_through_trait_object() -> TestResult {
        let mut receiver = FramedReceiver::new(&b"\x04\x00\x00\x00test\x05\x00\x00\x00hello"[..], 4);
        assert_eq!(receiver.recv().await?, Bytes::from_static(b"test"));
        assert_eq!(receiver.framed.read_buffer().as_ref(), b"\x05\x00\x00\x00hello");

        let receiver: Arc<Mutex<dyn FramedRecv + Send + Unpin>> = Arc::new(Mutex::new(receiver));
        let mut receiver = receiver.lock().await;
        receiver.set_max_frame_length(5);
        assert_eq!(receiver.recv().await?, Bytes::from_static(b"hello"));

        Ok(())
    }

    #[tokio::test]
    async fn lowered_limit_rejects_buffered_frame() -> TestResult {
        let mut receiver = FramedReceiver::new(&b"\x05\x00\x00\x00hello\x05\x00\x00\x00world"[..], 5);
        assert_eq!(receiver.recv().await?, Bytes::from_static(b"hello"));
        assert_eq!(receiver.framed.read_buffer().as_ref(), b"\x05\x00\x00\x00world");

        receiver.set_max_frame_length(4);
        assert!(receiver.recv().await.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn lowered_limit_preserves_frame_with_decoded_header() -> TestResult {
        let (mut writer, reader) = tokio::io::duplex(64);
        let mut receiver = FramedReceiver::new(reader, 5);
        writer.write_all(b"\x05\x00\x00\x00he").await?;
        {
            let mut recv = Box::pin(receiver.recv());
            assert!(futures_util::poll!(&mut recv).is_pending());
        }
        assert_eq!(receiver.framed.read_buffer().as_ref(), b"he");

        receiver.set_max_frame_length(4);
        writer.write_all(b"llo\x05\x00\x00\x00world").await?;
        assert_eq!(receiver.recv().await?, Bytes::from_static(b"hello"));
        assert!(receiver.recv().await.is_err());

        Ok(())
    }
}
