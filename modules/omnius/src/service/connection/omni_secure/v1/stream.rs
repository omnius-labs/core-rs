use std::{pin::Pin, vec};

use tokio::io::{AsyncRead, AsyncWrite};

use super::*;

const HEADER_SIZE: usize = 4;

#[allow(unused)]
pub enum OmniSecureStreamType {
    Connected,
    Accepted,
}

#[allow(unused)]
pub struct SecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    reader: R,
    writer: W,
    read_state: ReadState,
    write_state: WriteState,
    encoder: Aes256GcmEncoder,
}

enum ReadState {
    Init,
    Header { offset: usize, buf: [u8; HEADER_SIZE] },
    Body { offset: usize, buf: Vec<u8> },
}

#[allow(unused)]
enum WriteState {
    Init,
    Header { offset: usize, buf: [u8; HEADER_SIZE] },
    Body { offset: usize, buf: Vec<u8> },
}

#[allow(unused)]
impl<R, W> SecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub fn new(reader: R, writer: W, max_frame_length: usize) -> Self {
        Self {
            reader,
            writer,
            read_state: ReadState::Init,
            write_state: WriteState::Init,
            encoder: todo!(),
        }
    }

    pub fn handshake(&self) -> anyhow::Result<()> {
        todo!()
    }
}

impl<R, W> AsyncRead for SecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        root_buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            match this.read_state {
                ReadState::Init => {
                    this.read_state = ReadState::Header {
                        offset: 0,
                        buf: [0; HEADER_SIZE],
                    };
                }
                ReadState::Header { ref mut offset, ref mut buf } => {
                    if *offset == buf.len() {
                        let length = u32::from_le_bytes(*buf);
                        this.read_state = ReadState::Body {
                            offset: 0,
                            buf: vec![0; length as usize],
                        };
                        continue;
                    }

                    let mut tbuf = tokio::io::ReadBuf::new(&mut buf[*offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        other => return other,
                    };
                    *offset += n;
                }
                ReadState::Body { ref mut offset, ref mut buf } => {
                    if *offset == buf.len() {
                        let enc_buf = this.encoder.encode(buf);
                        root_buf.put_slice(&enc_buf);
                        this.read_state = ReadState::Init;
                        return std::task::Poll::Ready(Ok(()));
                    }

                    let mut tbuf = tokio::io::ReadBuf::new(&mut buf[*offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        other => return other,
                    };
                    *offset += n;
                }
            }
        }
    }
}

#[allow(unused)]
impl<R, W> AsyncWrite for SecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &[u8]) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}
