use std::backtrace::Backtrace;

use omnius_core_base::error::{OmniError, OmniErrorBuilder};

pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    backtrace: Option<Backtrace>,
}

pub struct ErrorBuilder {
    inner: Error,
}

impl Error {
    pub fn builder() -> ErrorBuilder {
        ErrorBuilder {
            inner: Self {
                kind: ErrorKind::Unknown,
                message: None,
                source: None,
                backtrace: None,
            },
        }
    }
}

impl OmniError for Error {
    type ErrorKind = ErrorKind;

    fn kind(&self) -> &Self::ErrorKind {
        &self.kind
    }

    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.backtrace.as_ref()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|s| &**s as &(dyn std::error::Error + 'static))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        OmniError::fmt(self, f)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        OmniError::fmt(self, f)
    }
}

impl OmniErrorBuilder<Error> for ErrorBuilder {
    type ErrorKind = ErrorKind;

    fn kind(mut self, kind: Self::ErrorKind) -> Self {
        self.inner.kind = kind;
        self
    }

    fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.inner.message = Some(message.into());
        self
    }

    fn source<E: Into<Box<dyn std::error::Error + Send + Sync>>>(mut self, source: E) -> Self {
        self.inner.source = Some(source.into());
        self
    }

    fn backtrace(mut self) -> Self {
        self.inner.backtrace = Some(Backtrace::capture());
        self
    }

    fn build(self) -> Error {
        self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Unknown,
    SerdeError,
    IoError,
    UnexpectedError,

    InvalidFormat,
    EndOfStream,
    UnsupportedVersion,
    UnsupportedType,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Unknown => write!(fmt, "unknown"),
            ErrorKind::SerdeError => write!(fmt, "serde error"),
            ErrorKind::IoError => write!(fmt, "io error"),
            ErrorKind::UnexpectedError => write!(fmt, "unexpected error"),

            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
            ErrorKind::EndOfStream => write!(fmt, "end of stream"),
            ErrorKind::UnsupportedVersion => write!(fmt, "unsupported version"),
            ErrorKind::UnsupportedType => write!(fmt, "unsupported type"),
        }
    }
}

impl From<omnius_core_rocketpack::Error> for Error {
    fn from(e: omnius_core_rocketpack::Error) -> Error {
        Error::builder()
            .kind(ErrorKind::SerdeError)
            .message("rocket pack error")
            .source(e)
            .build()
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::builder().kind(ErrorKind::IoError).message("io error").source(e).build()
    }
}

impl From<ed25519_dalek::pkcs8::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::Error) -> Self {
        Error::builder().kind(ErrorKind::InvalidFormat).message("pkcs8 error").source(e).build()
    }
}

impl From<ed25519_dalek::pkcs8::spki::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::spki::Error) -> Self {
        Error::builder()
            .kind(ErrorKind::InvalidFormat)
            .message("pkcs8 spki error")
            .source(e)
            .build()
    }
}

impl<T> From<nom::Err<nom::error::Error<T>>> for Error {
    fn from(e: nom::Err<nom::error::Error<T>>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::builder().kind(ErrorKind::InvalidFormat).message("nom incomplete").build(),
            nom::Err::Error(e) => Error::builder()
                .kind(ErrorKind::InvalidFormat)
                .message(format!("nom error: {:?}", e.code))
                .build(),
            nom::Err::Failure(e) => Error::builder()
                .kind(ErrorKind::InvalidFormat)
                .message(format!("nom failure: {:?}", e.code))
                .build(),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::builder()
            .kind(ErrorKind::InvalidFormat)
            .message("int parse error")
            .source(e)
            .build()
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Error {
        Error::builder()
            .kind(ErrorKind::InvalidFormat)
            .message("addr parse error")
            .source(e)
            .build()
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::builder()
            .kind(ErrorKind::InvalidFormat)
            .message("hex decode error")
            .source(e)
            .build()
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::builder()
            .kind(ErrorKind::InvalidFormat)
            .message("base64 decode error")
            .source(e)
            .build()
    }
}
