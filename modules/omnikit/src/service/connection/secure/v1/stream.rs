use std::{pin::Pin, sync::Arc, vec};

use chrono::Utc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};
use tracing::trace;

use omnius_core_base::clock::Clock;

use crate::{
    connection::{
        framed::{FramedReceiver, FramedSender},
        secure::OmniSecureStreamVersion,
    },
    OmniSigner,
};

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
    sign: Option<String>,
    encoder: Aes256GcmEncoder,
    decoder: Aes256GcmDecoder,
}

#[derive(Debug)]
enum ReadState {
    Init,
    Header { header_offset: usize, header_buf: [u8; HEADER_SIZE] },
    Body { body_offset: usize, body_buf: Vec<u8> },
}

#[derive(Debug)]
enum WriteState {
    Init,
    Payload {
        header: WritePayloadHeader,
        body: WritePayloadBody,
        length: usize,
    },
}

#[derive(Debug)]
struct WritePayloadHeader {
    offset: usize,
    buf: [u8; HEADER_SIZE],
}

#[derive(Debug)]
struct WritePayloadBody {
    offset: usize,
    buf: Vec<u8>,
}

#[allow(unused)]
impl<R, W> SecureStream<R, W>
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
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));
        let authenticator = Authenticator::new(OmniSecureStreamVersion::V1, stream_type, receiver.clone(), sender.clone(), signer, clock).await?;
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

impl<R, W> AsyncRead for SecureStream<R, W>
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
                    this.read_state = ReadState::Header {
                        header_offset: 0,
                        header_buf: [0; HEADER_SIZE],
                    };
                }
                ReadState::Header {
                    ref mut header_offset,
                    ref mut header_buf,
                } => {
                    let mut tbuf = tokio::io::ReadBuf::new(&mut header_buf[*header_offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        other => return other,
                    };
                    *header_offset += n;

                    if *header_offset == header_buf.len() {
                        let length = u32::from_le_bytes(*header_buf);
                        this.read_state = ReadState::Body {
                            body_offset: 0,
                            body_buf: vec![0; length as usize],
                        };
                    }
                }
                ReadState::Body {
                    ref mut body_offset,
                    ref mut body_buf,
                } => {
                    let mut tbuf = tokio::io::ReadBuf::new(&mut body_buf[*body_offset..]);
                    let n = match tokio::io::AsyncRead::poll_read(Pin::new(&mut this.reader), cx, &mut tbuf) {
                        std::task::Poll::Ready(Ok(())) => tbuf.filled().len(),
                        other => return other,
                    };
                    *body_offset += n;

                    if *body_offset == body_buf.len() {
                        let dec_buf = match this.decoder.decode(body_buf) {
                            Ok(buf) => buf,
                            Err(e) => return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
                        };
                        read_buf.put_slice(&dec_buf);
                        this.read_state = ReadState::Init;
                        return std::task::Poll::Ready(Ok(()));
                    }
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
    fn poll_write(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, write_buf: &[u8]) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = Pin::into_inner(self);
        loop {
            trace!("poll_write: {:?}", this.write_state);
            match &mut this.write_state {
                WriteState::Init => {
                    let enc_buf = match this.encoder.encode(write_buf) {
                        Ok(buf) => buf,
                        Err(e) => return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
                    };
                    this.write_state = WriteState::Payload {
                        header: WritePayloadHeader {
                            offset: 0,
                            buf: (enc_buf.len() as u32).to_le_bytes(),
                        },
                        body: WritePayloadBody { offset: 0, buf: enc_buf },
                        length: write_buf.len(),
                    };
                }
                WriteState::Payload {
                    ref mut header,
                    ref mut body,
                    length: ref plaintext_length,
                } => {
                    if header.offset < header.buf.len() {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &header.buf[header.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            other => return other,
                        };
                        header.offset += n;
                    } else {
                        let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &body.buf[body.offset..]) {
                            std::task::Poll::Ready(Ok(n)) => n,
                            other => return other,
                        };
                        body.offset += n;

                        if body.offset == body.buf.len() {
                            let plaintext_length = *plaintext_length;
                            this.write_state = WriteState::Init;

                            return std::task::Poll::Ready(Ok(plaintext_length));
                        }
                    }
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        tokio::io::AsyncWrite::poll_flush(Pin::new(&mut this.writer), cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut this.writer), cx)
    }
}
