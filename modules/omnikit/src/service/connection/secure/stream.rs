use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use parking_lot::Mutex;
use rand_core::CryptoRng;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, ReadHalf, WriteHalf};
use zeroize::Zeroizing;

use super::{
    auth::Authenticator,
    record::{RecordHeader, RecordKind, TrafficState},
    settings::*,
};
use crate::{generated::omni_sign::OmniCert, prelude::Result};

struct Transport<T> {
    reader: ReadHalf<T>,
    writer: WriteHalf<T>,
}

enum ReadState {
    Header { offset: usize, bytes: [u8; RecordHeader::LENGTH] },
    Body { offset: usize, header: RecordHeader, bytes: Vec<u8> },
    Plaintext { offset: usize, bytes: Zeroizing<Vec<u8>> },
    Closed,
}

impl ReadState {
    fn header() -> Self {
        Self::Header {
            offset: 0,
            bytes: [0; RecordHeader::LENGTH],
        }
    }
}

struct PendingRecord {
    kind: RecordKind,
    offset: usize,
    bytes: Vec<u8>,
}

/// 相互署名であっても、期待する公開鍵の照合は application 利用前に呼び出し側が行う。
pub struct OmniSecureStream<T> {
    transport: Option<Transport<T>>,
    encoder: Option<TrafficState>,
    decoder: Option<TrafficState>,
    option: OmniSecureStreamOption,
    peer_cert: Option<OmniCert>,
    transcript: [u8; 32],
    read_state: ReadState,
    pending: Option<PendingRecord>,
    closing: bool,
    close_queued: bool,
    lower_shutdown_started: bool,
    shutdown_complete: bool,
    failed: bool,
    reader_waker: Option<Waker>,
    writer_waker: Option<Waker>,
}

impl<T> OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    pub async fn new(stream: T, typ: OmniSecureStreamType, option: OmniSecureStreamOption, auth: OmniSecureAuth, rng: Arc<Mutex<dyn CryptoRng + Send + Sync>>) -> Result<Self> {
        option.validate()?;
        let (mut reader, mut writer) = tokio::io::split(stream);
        let result = Authenticator::authenticate(&mut reader, &mut writer, typ, &auth, &option, rng).await?;
        let (send_direction, recv_direction) = match typ {
            OmniSecureStreamType::Connected => (1, 2),
            OmniSecureStreamType::Accepted => (2, 1),
        };
        Ok(Self {
            transport: Some(Transport { reader, writer }),
            encoder: Some(TrafficState::new(result.send_secret, result.transcript, send_direction)),
            decoder: Some(TrafficState::new(result.recv_secret, result.transcript, recv_direction)),
            option,
            peer_cert: result.peer_cert,
            transcript: result.transcript,
            read_state: ReadState::header(),
            pending: None,
            closing: false,
            close_queued: false,
            lower_shutdown_started: false,
            shutdown_complete: false,
            failed: false,
            reader_waker: None,
            writer_waker: None,
        })
    }

    pub fn peer_cert(&self) -> Option<&OmniCert> {
        self.peer_cert.as_ref()
    }
    pub fn peer_public_key(&self) -> Option<&[u8]> {
        self.peer_cert.as_ref().map(|cert| cert.public_key.as_slice())
    }
    pub fn handshake_hash(&self) -> &[u8; 32] {
        &self.transcript
    }

    #[cfg(test)]
    pub(super) fn set_test_send_state(&mut self, generation: u64, sequence: u64, bytes_used: u64) {
        self.encoder.as_mut().unwrap().set_test_state(generation, sequence, bytes_used);
    }

    #[cfg(test)]
    pub(super) fn set_test_recv_state(&mut self, generation: u64, sequence: u64, bytes_used: u64) {
        self.decoder.as_mut().unwrap().set_test_state(generation, sequence, bytes_used);
    }

    fn fail<U>(&mut self, error: io::Error) -> Poll<io::Result<U>> {
        self.failed = true;
        self.transport = None;
        self.encoder = None;
        self.decoder = None;
        self.pending = None;
        self.read_state = ReadState::Closed;
        if let Some(waker) = self.reader_waker.take() {
            waker.wake();
        }
        if let Some(waker) = self.writer_waker.take() {
            waker.wake();
        }
        Poll::Ready(Err(error))
    }

    fn terminal() -> io::Error {
        io::Error::new(io::ErrorKind::ConnectionAborted, "secure stream failed")
    }

    fn queue(&mut self, kind: RecordKind, plaintext: &[u8]) -> io::Result<()> {
        let encoder = self.encoder.as_mut().ok_or_else(Self::terminal)?;
        let bytes = encoder.encode(kind, plaintext)?;
        self.pending = Some(PendingRecord { kind, offset: 0, bytes });
        Ok(())
    }

    fn poll_drain(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while let Some(pending) = &mut self.pending {
            let Some(transport) = self.transport.as_mut() else {
                return Poll::Ready(Err(Self::terminal()));
            };
            match Pin::new(&mut transport.writer).poll_write(cx, &pending.bytes[pending.offset..]) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "secure record write made no progress"))),
                Poll::Ready(Ok(count)) => pending.offset += count,
            }
            if pending.offset == pending.bytes.len() {
                match pending.kind {
                    RecordKind::KeyUpdate => self.encoder.as_mut().ok_or_else(Self::terminal)?.advance()?,
                    RecordKind::Close => self.encoder = None,
                    RecordKind::Data => {}
                }
                self.pending = None;
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl<T> AsyncRead for OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.reader_waker = Some(cx.waker().clone());
        if this.failed {
            return Poll::Ready(Err(Self::terminal()));
        }
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            match &mut this.read_state {
                ReadState::Closed => return Poll::Ready(Ok(())),
                ReadState::Plaintext { offset, bytes } => {
                    let length = output.remaining().min(bytes.len() - *offset);
                    output.put_slice(&bytes[*offset..*offset + length]);
                    *offset += length;
                    if *offset == bytes.len() {
                        this.read_state = ReadState::header();
                    }
                    return Poll::Ready(Ok(()));
                }
                ReadState::Header { offset, bytes } => {
                    let Some(transport) = this.transport.as_mut() else {
                        return Poll::Ready(Err(Self::terminal()));
                    };
                    let mut buffer = ReadBuf::new(&mut bytes[*offset..]);
                    let count = match Pin::new(&mut transport.reader).poll_read(cx, &mut buffer) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(error)) => return this.fail(error),
                        Poll::Ready(Ok(())) => buffer.filled().len(),
                    };
                    if count == 0 {
                        return this.fail(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF before authenticated Close"));
                    }
                    *offset += count;
                    if *offset == bytes.len() {
                        let header = match RecordHeader::parse(bytes) {
                            Ok(value) => value,
                            Err(error) => return this.fail(error),
                        };
                        let Some(decoder) = &this.decoder else {
                            return Poll::Ready(Err(Self::terminal()));
                        };
                        if let Err(error) = decoder.validate_header(&header) {
                            return this.fail(error);
                        }
                        this.read_state = ReadState::Body {
                            offset: 0,
                            bytes: vec![0; header.plaintext_length + 16],
                            header,
                        };
                    }
                }
                ReadState::Body { offset, header, bytes } => {
                    let Some(transport) = this.transport.as_mut() else {
                        return Poll::Ready(Err(Self::terminal()));
                    };
                    let mut buffer = ReadBuf::new(&mut bytes[*offset..]);
                    let count = match Pin::new(&mut transport.reader).poll_read(cx, &mut buffer) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(error)) => return this.fail(error),
                        Poll::Ready(Ok(())) => buffer.filled().len(),
                    };
                    if count == 0 {
                        return this.fail(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated secure record"));
                    }
                    *offset += count;
                    if *offset == bytes.len() {
                        let Some(decoder) = this.decoder.as_mut() else {
                            return Poll::Ready(Err(Self::terminal()));
                        };
                        let plaintext = match decoder.decode(header, bytes) {
                            Ok(value) => value,
                            Err(error) => return this.fail(error),
                        };
                        this.read_state = match header.kind {
                            RecordKind::Data => ReadState::Plaintext { offset: 0, bytes: plaintext },
                            RecordKind::KeyUpdate => ReadState::header(),
                            RecordKind::Close => {
                                this.decoder = None;
                                ReadState::Closed
                            }
                        };
                    }
                }
            }
        }
    }
}

impl<T> AsyncWrite for OmniSecureStream<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, input: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.writer_waker = Some(cx.waker().clone());
        if this.failed {
            return Poll::Ready(Err(Self::terminal()));
        }
        if this.closing {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "secure stream is closing")));
        }
        if input.is_empty() {
            return Poll::Ready(Ok(0));
        }
        match this.poll_drain(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(error)) => return this.fail(error),
            Poll::Ready(Ok(())) => {}
        }
        let Some(encoder) = &this.encoder else {
            return Poll::Ready(Err(Self::terminal()));
        };
        if encoder.data_capacity(this.option.rekey_after_bytes, this.option.rekey_after_records) == 0 {
            let next = match encoder.next_generation() {
                Ok(next) => next,
                Err(error) => return this.fail(error),
            };
            if let Err(error) = this.queue(RecordKind::KeyUpdate, &next.to_le_bytes()) {
                return this.fail(error);
            }
            match this.poll_drain(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return this.fail(error),
                Poll::Ready(Ok(())) => {}
            }
        }
        let Some(encoder) = &this.encoder else {
            return Poll::Ready(Err(Self::terminal()));
        };
        let length = input.len().min(encoder.data_capacity(this.option.rekey_after_bytes, this.option.rekey_after_records));
        if let Err(error) = this.queue(RecordKind::Data, &input[..length]) {
            return this.fail(error);
        }
        Poll::Ready(Ok(length))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.writer_waker = Some(cx.waker().clone());
        if this.failed {
            return Poll::Ready(Err(Self::terminal()));
        }
        if this.lower_shutdown_started {
            return Poll::Ready(Ok(()));
        }
        match this.poll_drain(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(error)) => return this.fail(error),
            Poll::Ready(Ok(())) => {}
        }
        let Some(transport) = this.transport.as_mut() else {
            return Poll::Ready(Err(Self::terminal()));
        };
        match Pin::new(&mut transport.writer).poll_flush(cx) {
            Poll::Ready(Err(error)) => this.fail(error),
            result => result,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.writer_waker = Some(cx.waker().clone());
        if this.failed {
            return Poll::Ready(Err(Self::terminal()));
        }
        if this.shutdown_complete {
            return Poll::Ready(Ok(()));
        }
        this.closing = true;
        if !this.lower_shutdown_started {
            match this.poll_drain(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return this.fail(error),
                Poll::Ready(Ok(())) => {}
            }
            if !this.close_queued {
                if let Err(error) = this.queue(RecordKind::Close, &[]) {
                    return this.fail(error);
                }
                this.close_queued = true;
            }
            match this.poll_drain(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return this.fail(error),
                Poll::Ready(Ok(())) => {}
            }
            let Some(transport) = this.transport.as_mut() else {
                return Poll::Ready(Err(Self::terminal()));
            };
            match Pin::new(&mut transport.writer).poll_flush(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => return this.fail(error),
                Poll::Ready(Ok(())) => {}
            }
            this.lower_shutdown_started = true;
        }
        let Some(transport) = this.transport.as_mut() else {
            return Poll::Ready(Err(Self::terminal()));
        };
        match Pin::new(&mut transport.writer).poll_shutdown(cx) {
            Poll::Ready(Err(error)) => this.fail(error),
            Poll::Ready(Ok(())) => {
                this.shutdown_complete = true;
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
