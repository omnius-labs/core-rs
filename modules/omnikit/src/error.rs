use omnius_core_rocketpack::Error as RocketPackError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    RocketPackError,
    IoError,
    EndOfStream,
    InvalidFormat,
    UnsupportedVersion,
    UnsupportedType,
    UnexpectedError,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::RocketPackError => write!(fmt, "rocket pack error"),
            ErrorKind::IoError => write!(fmt, "io error"),
            ErrorKind::EndOfStream => write!(fmt, "end of stream"),
            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
            ErrorKind::UnsupportedVersion => write!(fmt, "unsupported version"),
            ErrorKind::UnsupportedType => write!(fmt, "unsupported type"),
            ErrorKind::UnexpectedError => write!(fmt, "unexpected error"),
        }
    }
}

pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: None,
        }
    }

    pub fn message<S: AsRef<str>>(mut self, message: S) -> Self {
        self.message = Some(message.as_ref().to_string());
        self
    }

    pub fn source<E: Into<Box<dyn std::error::Error + Send + Sync>>>(mut self, source: E) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            write!(fmt, "{}: {}", self.kind, message)
        } else {
            write!(fmt, "{}", self.kind)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|s| &**s as &(dyn std::error::Error + 'static))
    }
}

impl From<RocketPackError> for Error {
    fn from(e: RocketPackError) -> Error {
        Error::new(ErrorKind::RocketPackError).message("rocket pack error").source(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::new(ErrorKind::IoError).message("io error").source(e)
    }
}

impl From<ed25519_dalek::pkcs8::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::Error) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("pkcs8 error").source(e)
    }
}

impl From<ed25519_dalek::pkcs8::spki::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::spki::Error) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("pkcs8 spki error").source(e)
    }
}

impl<T> From<nom::Err<nom::error::Error<T>>> for Error {
    fn from(e: nom::Err<nom::error::Error<T>>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::new(ErrorKind::InvalidFormat).message("nom incomplete"),
            nom::Err::Error(e) => Error::new(ErrorKind::InvalidFormat).message(format!("nom error: {:?}", e.code)),
            nom::Err::Failure(e) => Error::new(ErrorKind::InvalidFormat).message(format!("nom failure: {:?}", e.code)),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::new(ErrorKind::InvalidFormat).message("int parse error").source(e)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Error {
        Error::new(ErrorKind::InvalidFormat).message("addr parse error").source(e)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("hex decode error").source(e)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("base64 decode error").source(e)
    }
}
