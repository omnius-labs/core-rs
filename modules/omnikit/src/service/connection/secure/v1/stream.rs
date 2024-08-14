use std::{pin::Pin, sync::Arc, vec};

use chrono::Utc;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};
use tokio_util::bytes::{Buf as _, Bytes, BytesMut};
use tracing::trace;

use omnius_core_base::{clock::Clock, random_bytes::RandomBytesProvider};

use crate::{
    connection::framed::{FramedReceiver, FramedSender},
    OmniSigner,
};

use super::*;

const HEADER_SIZE: usize = 4;
const MAX_FRAME_LENGTH: usize = 1024 * 64;

#[allow(unused)]
pub enum OmniSecureStreamType {
    Connected,
    Accepted,
}

#[allow(unused)]
pub struct OmniSecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    reader: R,
    writer: W,
    read_state: ReadState,
    write_state: WriteState,
    sign: Option<String>,
    encoder: Aes256GcmEncoder,
    decoder: Aes256GcmDecoder,
}

#[derive(Debug)]
enum ReadState {
    Init,
    ReceiveHeader { header_offset: usize, header_buf: [u8; HEADER_SIZE] },
    ReceiveBody { body_offset: usize, body_buf: Vec<u8> },
    ReadPlaintext { plaintext: Bytes },
}

#[derive(Debug)]
enum WriteState {
    Init,
    WritePlaintext { plaintext: BytesMut },
    SendPayload { header: SendHeader, body: SendBody },
}

#[derive(Debug)]
struct SendHeader {
    offset: usize,
    buf: [u8; HEADER_SIZE],
}

#[derive(Debug)]
struct SendBody {
    offset: usize,
    buf: Vec<u8>,
}

#[allow(unused)]
impl<R, W> OmniSecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub async fn new(
        reader: R,
        writer: W,
        stream_type: OmniSecureStreamType,
        max_frame_length: usize,
        signer: Option<OmniSigner>,
        random_bytes_provider: Arc<Mutex<dyn RandomBytesProvider + Send + Sync>>,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));
        let authenticator = Authenticator::new(stream_type, receiver.clone(), sender.clone(), signer, random_bytes_provider, clock).await?;
        let auth_result = authenticator.auth().await?;
        drop(authenticator);

        let reader = Arc::try_unwrap(receiver)
            .map_err(|_| anyhow::anyhow!("unexpected result"))?
            .into_inner()
            .into_inner();
        let writer = Arc::try_unwrap(sender)
            .map_err(|_| anyhow::anyhow!("unexpected result"))?
            .into_inner()
            .into_inner();

        Ok(Self {
            reader,
            writer,
            read_state: ReadState::Init,
            write_state: WriteState::Init,
            sign: auth_result.sign,
            encoder: Aes256GcmEncoder::new(&auth_result.enc_key, &auth_result.enc_nonce),
            decoder: Aes256GcmDecoder::new(&auth_result.dec_key, &auth_result.dec_nonce),
        })
    }

    pub fn sign(&self) -> Option<&str> {
        self.sign.as_deref()
    }
}

impl<R, W> AsyncRead for OmniSecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        read_buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            trace!("poll_read: {:?}", this.read_state);
            match this.read_state {
                ReadState::Init => {
                    this.read_state = ReadState::ReceiveHeader {
                        header_offset: 0,
                        header_buf: [0; HEADER_SIZE],
                    };
                }
                ReadState::ReceiveHeader {
                    ref mut header_offset,
                    ref mut header_buf,
                } => {
                    let mut tbuf = tokio::io::ReadBuf::new(&mut header_buf[*header_offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                        std::task::Poll::Pending => return std::task::Poll::Pending,
                    };
                    *header_offset += n;

                    if *header_offset == header_buf.len() {
                        let length = u32::from_le_bytes(*header_buf);

                        if length > (MAX_FRAME_LENGTH + 16) as u32 {
                            return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "frame length is too long")));
                        }

                        this.read_state = ReadState::ReceiveBody {
                            body_offset: 0,
                            body_buf: vec![0; length as usize],
                        };
                    }
                }
                ReadState::ReceiveBody {
                    ref mut body_offset,
                    ref mut body_buf,
                } => {
                    let mut tbuf = tokio::io::ReadBuf::new(&mut body_buf[*body_offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                        std::task::Poll::Pending => return std::task::Poll::Pending,
                    };
                    *body_offset += n;

                    if *body_offset == body_buf.len() {
                        let dec_buf = match this.decoder.decode(body_buf) {
                            Ok(buf) => buf,
                            Err(e) => return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
                        };
                        this.read_state = ReadState::ReadPlaintext {
                            plaintext: Bytes::from(dec_buf),
                        };
                    }
                }
                ReadState::ReadPlaintext { ref mut plaintext } => {
                    let size = std::cmp::min(plaintext.len(), read_buf.remaining());
                    read_buf.put_slice(plaintext.slice(0..size).as_ref());
                    plaintext.advance(size);

                    if plaintext.is_empty() {
                        this.read_state = ReadState::Init;
                    }

                    return std::task::Poll::Ready(Ok(()));
                }
            }
        }
    }
}

#[allow(unused)]
impl<R, W> AsyncWrite for OmniSecureStream<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, write_buf: &[u8]) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = Pin::into_inner(self);
        loop {
            trace!("poll_write: {:?}", this.write_state);
            match &mut this.write_state {
                WriteState::Init => {
                    this.write_state = WriteState::WritePlaintext { plaintext: BytesMut::new() };
                }
                WriteState::WritePlaintext { ref mut plaintext } => {
                    let size = std::cmp::min(MAX_FRAME_LENGTH - plaintext.len(), write_buf.len());
                    plaintext.extend_from_slice(&write_buf[..size]);

                    if plaintext.len() == MAX_FRAME_LENGTH {
                        let enc_buf = match this.encoder.encode(plaintext) {
                            Ok(buf) => buf,
                            Err(e) => return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
                        };
                        this.write_state = WriteState::SendPayload {
                            header: SendHeader {
                                offset: 0,
                                buf: (enc_buf.len() as u32).to_le_bytes(),
                            },
                            body: SendBody { offset: 0, buf: enc_buf },
                        };
                    }

                    return std::task::Poll::Ready(Ok(size));
                }
                WriteState::SendPayload {
                    ref mut header,
                    ref mut body,
                } => {
                    if header.offset < header.buf.len() {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &header.buf[header.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                            std::task::Poll::Pending => return std::task::Poll::Pending,
                        };
                        header.offset += n;
                    } else {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &body.buf[body.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                            std::task::Poll::Pending => return std::task::Poll::Pending,
                        };
                        body.offset += n;

                        if body.offset == body.buf.len() {
                            this.write_state = WriteState::Init;
                        }
                    }
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        trace!("poll_flush: {:?}", this.write_state);
        loop {
            match &mut this.write_state {
                WriteState::Init => {
                    return tokio::io::AsyncWrite::poll_flush(Pin::new(&mut this.writer), cx);
                }
                WriteState::WritePlaintext { ref mut plaintext } => {
                    let enc_buf = match this.encoder.encode(plaintext) {
                        Ok(buf) => buf,
                        Err(e) => return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
                    };
                    this.write_state = WriteState::SendPayload {
                        header: SendHeader {
                            offset: 0,
                            buf: (enc_buf.len() as u32).to_le_bytes(),
                        },
                        body: SendBody { offset: 0, buf: enc_buf },
                    };
                }
                WriteState::SendPayload {
                    ref mut header,
                    ref mut body,
                } => {
                    if header.offset < header.buf.len() {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &header.buf[header.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                            std::task::Poll::Pending => return std::task::Poll::Pending,
                        };
                        header.offset += n;
                    } else {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &body.buf[body.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                            std::task::Poll::Pending => return std::task::Poll::Pending,
                        };
                        body.offset += n;

                        if body.offset == body.buf.len() {
                            this.write_state = WriteState::Init;
                        }
                    }
                }
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        trace!("poll_shutdown: {:?}", this.write_state);
        tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut this.writer), cx)
    }
}
