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
            backtrace: Backtrace::disabled(),
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
    IoError,
    TimeError,

    AwsError,
    GcpError,

    InvalidFormat,
    NotFound,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Unknown => write!(fmt, "unknown"),
            ErrorKind::IoError => write!(fmt, "io error"),
            ErrorKind::TimeError => write!(fmt, "time conversion error"),

            ErrorKind::AwsError => write!(fmt, "aws error"),
            ErrorKind::GcpError => write!(fmt, "gcp error"),

            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
            ErrorKind::NotFound => write!(fmt, "not found"),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        Error::new(ErrorKind::Unknown)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::from_error(e, ErrorKind::IoError).with_message("io operation failed")
    }
}

impl From<chrono::OutOfRangeError> for Error {
    fn from(e: chrono::OutOfRangeError) -> Self {
        Error::from_error(e, ErrorKind::TimeError).with_message("out of range")
    }
}

#[cfg(feature = "aws")]
impl<E, R> From<aws_smithy_runtime_api::client::result::SdkError<E, R>> for Error
where
    E: std::error::Error + Send + Sync + 'static,
    R: std::fmt::Debug + Send + Sync + 'static,
{
    fn from(e: aws_smithy_runtime_api::client::result::SdkError<E, R>) -> Self {
        Error::from_error(e, ErrorKind::AwsError).with_message("aws sdk operation failed")
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::primitives::ByteStreamError> for Error {
    fn from(e: aws_sdk_s3::primitives::ByteStreamError) -> Self {
        Error::from_error(e, ErrorKind::AwsError).with_message("aws s3 byte stream error")
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::presigning::PresigningConfigError> for Error {
    fn from(e: aws_sdk_s3::presigning::PresigningConfigError) -> Self {
        Error::from_error(e, ErrorKind::AwsError).with_message("aws s3 presigning config error")
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::error::BuildError> for Error {
    fn from(e: aws_sdk_s3::error::BuildError) -> Self {
        Error::from_error(e, ErrorKind::AwsError).with_message("aws s3 build error")
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::error::Error> for Error {
    fn from(e: gcloud_sdk::error::Error) -> Self {
        Error::from_error(e, ErrorKind::GcpError).with_message("gcp sdk operation failed")
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::tonic::Status> for Error {
    fn from(e: gcloud_sdk::tonic::Status) -> Self {
        Error::from_error(e, ErrorKind::GcpError).with_message("gcp sdk operation failed")
    }
}
