use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::compat::FuturesAsyncReadCompatExt as _;

pub struct YamuxStream {
    stream_id: u32,
    reader: tokio::io::ReadHalf<tokio_util::compat::Compat<yamux::Stream>>,
    writer: tokio::io::WriteHalf<tokio_util::compat::Compat<yamux::Stream>>,
    stream_count: Arc<AtomicUsize>,
}

impl YamuxStream {
    pub(crate) fn new(stream: yamux::Stream, stream_count: Arc<AtomicUsize>) -> Self {
        stream_count.fetch_add(1, Ordering::SeqCst);

        let stream_id = stream.id().val();
        let (reader, writer) = tokio::io::split(stream.compat());

        Self {
            stream_id,
            reader,
            writer,
            stream_count,
        }
    }

    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }
}

impl Drop for YamuxStream {
    fn drop(&mut self) {
        self.stream_count.fetch_sub(1, Ordering::SeqCst);
    }
}

impl AsyncRead for YamuxStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, read_buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.reader).poll_read(cx, read_buf)
    }
}

impl AsyncWrite for YamuxStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, write_buf: &[u8]) -> Poll<std::result::Result<usize, std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.writer).poll_write(cx, write_buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.writer).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.writer).poll_shutdown(cx)
    }
}
