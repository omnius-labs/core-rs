use std::{pin::Pin, sync::Arc, time::Duration, vec};

use chrono::Utc;
use parking_lot::Mutex;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::bytes::{Buf as _, Bytes, BytesMut};
use tracing::trace;

use omnius_core_base::clock::Clock;

use crate::{
    generated::omni_sign::{OmniCert, OmniSigner},
    prelude::*,
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
pub struct OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    reader: ReadHalf<T>,
    writer: WriteHalf<T>,
    read_state: ReadState,
    write_state: WriteState,
    sign_id: Option<String>,
    peer_cert: Option<OmniCert>,
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
impl<T> OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        stream: T,
        stream_type: OmniSecureStreamType,
        max_frame_length: usize,
        signer: Option<OmniSigner>,
        require_peer_signature: bool,
        max_clock_skew: Duration,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
        rng: Arc<Mutex<dyn rand::Rng + Send + Sync>>,
    ) -> Result<Self> {
        let (reader, writer) = tokio::io::split(stream);
        let mut authenticator = Authenticator::new(stream_type, reader, writer, max_frame_length, signer, require_peer_signature, max_clock_skew, clock, rng).await?;
        let auth_result = authenticator.auth().await?;
        let (reader, writer) = authenticator.into_inner();

        Ok(Self {
            reader,
            writer,
            read_state: ReadState::Init,
            write_state: WriteState::Init,
            sign_id: auth_result.peer_cert.as_ref().map(ToString::to_string),
            peer_cert: auth_result.peer_cert,
            encoder: Aes256GcmEncoder::new(&auth_result.enc_key, &auth_result.enc_nonce),
            decoder: Aes256GcmDecoder::new(&auth_result.dec_key, &auth_result.dec_nonce),
        })
    }

    pub fn sign_id(&self) -> Option<&str> {
        self.sign_id.as_deref()
    }

    pub fn peer_cert(&self) -> Option<&OmniCert> {
        self.peer_cert.as_ref()
    }
}

impl<T> AsyncRead for OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    fn poll_read(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, read_buf: &mut tokio::io::ReadBuf<'_>) -> std::task::Poll<std::io::Result<()>> {
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
                    if n == 0 {
                        return if *header_offset == 0 {
                            std::task::Poll::Ready(Ok(()))
                        } else {
                            std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "stream ended during frame header")))
                        };
                    }
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
                    if n == 0 && *body_offset < body_buf.len() {
                        return std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "stream ended during frame body")));
                    }
                    *body_offset += n;

                    if *body_offset == body_buf.len() {
                        let dec_buf = match this.decoder.decode(body_buf) {
                            Ok(buf) => buf,
                            Err(e) => return std::task::Poll::Ready(Err(std::io::Error::other(e.to_string()))),
                        };
                        this.read_state = ReadState::ReadPlaintext { plaintext: Bytes::from(dec_buf) };
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

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use tokio::{
        io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream},
        time::timeout,
    };

    use super::*;

    fn stream_pair() -> (OmniSecureStream<DuplexStream>, DuplexStream) {
        let (stream, peer) = tokio::io::duplex(4096);
        let (reader, writer) = tokio::io::split(stream);
        let secure = OmniSecureStream {
            reader,
            writer,
            read_state: ReadState::Init,
            write_state: WriteState::Init,
            sign_id: None,
            peer_cert: None,
            encoder: Aes256GcmEncoder::new(&[0; 32], &[0; 12]),
            decoder: Aes256GcmDecoder::new(&[0; 32], &[0; 12]),
        };
        (secure, peer)
    }

    #[tokio::test]
    async fn eof_at_frame_boundary_remains_eof() {
        for send_frame in [false, true] {
            let (mut stream, mut peer) = stream_pair();
            let plaintext = b"hello";
            if send_frame {
                let body = Aes256GcmEncoder::new(&[0; 32], &[0; 12]).encode(plaintext).unwrap();
                peer.write_all(&(body.len() as u32).to_le_bytes()).await.unwrap();
                peer.write_all(&body).await.unwrap();
            }
            drop(peer);

            if send_frame {
                // Read the plaintext in parts to exercise the return to a frame boundary.
                for expected in plaintext.chunks(2) {
                    let mut buffer = [0; 2];
                    let n = timeout(Duration::from_secs(1), stream.read(&mut buffer)).await.expect("frame read timed out").unwrap();
                    assert_eq!(&buffer[..n], expected);
                }
            }

            for _ in 0..3 {
                let mut buffer = [0xaa; 2];
                let n = timeout(Duration::from_secs(1), stream.read(&mut buffer)).await.expect("EOF read timed out").unwrap();
                assert_eq!(n, 0);
                assert_eq!(buffer, [0xaa; 2]);
            }
        }
    }

    #[tokio::test]
    async fn eof_during_frame_header_is_unexpected_eof() {
        for length in 1..HEADER_SIZE {
            let (mut stream, mut peer) = stream_pair();
            peer.write_all(&32u32.to_le_bytes()[..length]).await.unwrap();
            drop(peer);

            let mut buffer = [0xaa; 8];
            let error = timeout(Duration::from_secs(1), stream.read(&mut buffer))
                .await
                .expect("partial header read timed out")
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::UnexpectedEof);
            assert_eq!(buffer, [0xaa; 8]);
        }
    }

    #[tokio::test]
    async fn eof_during_frame_body_is_unexpected_eof() {
        let body = Aes256GcmEncoder::new(&[0; 32], &[0; 12]).encode(b"hello").unwrap();
        for length in 0..body.len() {
            let (mut stream, mut peer) = stream_pair();
            peer.write_all(&(body.len() as u32).to_le_bytes()).await.unwrap();
            peer.write_all(&body[..length]).await.unwrap();
            drop(peer);

            let mut buffer = [0xaa; 8];
            let error = timeout(Duration::from_secs(1), stream.read(&mut buffer))
                .await
                .expect("partial body read timed out")
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::UnexpectedEof);
            assert_eq!(buffer, [0xaa; 8]);
        }
    }
}

#[allow(unused)]
impl<T> AsyncWrite for OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, write_buf: &[u8]) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        let this = Pin::into_inner(self);
        loop {
            trace!("poll_write: {:?}", this.write_state);
            match &mut this.write_state {
                WriteState::Init => {
                    this.write_state = WriteState::WritePlaintext { plaintext: BytesMut::new() };
                }
                WriteState::WritePlaintext { plaintext } => {
                    let size = std::cmp::min(MAX_FRAME_LENGTH - plaintext.len(), write_buf.len());
                    plaintext.extend_from_slice(&write_buf[..size]);

                    if plaintext.len() == MAX_FRAME_LENGTH {
                        let enc_buf = match this.encoder.encode(plaintext) {
                            Ok(buf) => buf,
                            Err(e) => return std::task::Poll::Ready(Err(std::io::Error::other(e.to_string()))),
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
                WriteState::SendPayload { header, body } => {
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

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        trace!("poll_flush: {:?}", this.write_state);
        loop {
            match &mut this.write_state {
                WriteState::Init => {
                    return tokio::io::AsyncWrite::poll_flush(Pin::new(&mut this.writer), cx);
                }
                WriteState::WritePlaintext { plaintext } => {
                    let enc_buf = match this.encoder.encode(plaintext) {
                        Ok(buf) => buf,
                        Err(e) => return std::task::Poll::Ready(Err(std::io::Error::other(e.to_string()))),
                    };
                    this.write_state = WriteState::SendPayload {
                        header: SendHeader {
                            offset: 0,
                            buf: (enc_buf.len() as u32).to_le_bytes(),
                        },
                        body: SendBody { offset: 0, buf: enc_buf },
                    };
                }
                WriteState::SendPayload { header, body } => {
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

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        let this = Pin::into_inner(self);
        trace!("poll_shutdown: {:?}", this.write_state);
        tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut this.writer), cx)
    }
}
