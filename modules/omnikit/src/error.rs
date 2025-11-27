use std::backtrace::Backtrace;

use omnius_core_base::error::OmniError;

pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    backtrace: Backtrace,
}

impl OmniError for Error {
    type ErrorKind = ErrorKind;

    fn new(kind: Self::ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    fn from_error<E: Into<Box<dyn std::error::Error + Send + Sync>>>(source: E, kind: Self::ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: Some(source.into()),
            backtrace: Backtrace::capture(),
        }
    }

    fn kind(&self) -> &Self::ErrorKind {
        &self.kind
    }

    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Unknown,
    SerdeError,
    IoError,
    UnexpectedError,

    InvalidFormat,
    EndOfStream,
    UnsupportedType,
    AlreadyConnected,
    NotConnected,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Unknown => write!(fmt, "unknown"),
            ErrorKind::SerdeError => write!(fmt, "serde_error"),
            ErrorKind::IoError => write!(fmt, "io_error"),
            ErrorKind::UnexpectedError => write!(fmt, "unexpected_error"),

            ErrorKind::InvalidFormat => write!(fmt, "invalid_format"),
            ErrorKind::EndOfStream => write!(fmt, "end_of_stream"),
            ErrorKind::UnsupportedType => write!(fmt, "unsupported_type"),
            ErrorKind::AlreadyConnected => write!(fmt, "already_connected"),
            ErrorKind::NotConnected => write!(fmt, "not_connected"),
        }
    }
}

impl From<omnius_core_rocketpack::RocketPackEncoderError> for Error {
    fn from(e: omnius_core_rocketpack::RocketPackEncoderError) -> Error {
        Error::from_error(e, ErrorKind::SerdeError).with_message("rocket pack encode error")
    }
}

impl From<omnius_core_rocketpack::RocketPackDecoderError> for Error {
    fn from(e: omnius_core_rocketpack::RocketPackDecoderError) -> Error {
        Error::from_error(e, ErrorKind::SerdeError).with_message("rocket pack decode error")
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::from_error(e, ErrorKind::IoError).with_message("io error")
    }
}

impl From<ed25519_dalek::pkcs8::Error> for Error {
    fn from(_: ed25519_dalek::pkcs8::Error) -> Self {
        Error::new(ErrorKind::InvalidFormat).with_message("pkcs8 error")
    }
}

impl From<ed25519_dalek::pkcs8::spki::Error> for Error {
    fn from(_: ed25519_dalek::pkcs8::spki::Error) -> Self {
        Error::new(ErrorKind::InvalidFormat).with_message("pkcs8 spki error")
    }
}

impl<T> From<nom::Err<nom::error::Error<T>>> for Error {
    fn from(e: nom::Err<nom::error::Error<T>>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::new(ErrorKind::InvalidFormat).with_message("nom incomplete"),
            nom::Err::Error(e) => Error::new(ErrorKind::InvalidFormat).with_message(format!("nom error: {:?}", e.code)),
            nom::Err::Failure(e) => Error::new(ErrorKind::InvalidFormat).with_message(format!("nom failure: {:?}", e.code)),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("int parse error")
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Error {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("addr parse error")
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("hex decode error")
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("base64 decode error")
    }
}
