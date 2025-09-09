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
        Error::builder().build()
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::builder().kind(ErrorKind::IoError).message("io operation failed").source(e).build()
    }
}

impl From<chrono::OutOfRangeError> for Error {
    fn from(e: chrono::OutOfRangeError) -> Self {
        Error::builder().kind(ErrorKind::TimeError).message("out of range").source(e).build()
    }
}

#[cfg(feature = "aws")]
impl<E, R> From<aws_smithy_runtime_api::client::result::SdkError<E, R>> for Error
where
    E: std::error::Error + Send + Sync + 'static,
    R: std::fmt::Debug + Send + Sync + 'static,
{
    fn from(e: aws_smithy_runtime_api::client::result::SdkError<E, R>) -> Self {
        Error::builder().kind(ErrorKind::AwsError).message("aws sdk operation failed").source(e).build()
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::primitives::ByteStreamError> for Error {
    fn from(e: aws_sdk_s3::primitives::ByteStreamError) -> Self {
        Error::builder().kind(ErrorKind::AwsError).message("aws s3 byte stream error").source(e).build()
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::presigning::PresigningConfigError> for Error {
    fn from(e: aws_sdk_s3::presigning::PresigningConfigError) -> Self {
        Error::builder().kind(ErrorKind::AwsError).message("aws s3 presigning config error").source(e).build()
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::error::BuildError> for Error {
    fn from(e: aws_sdk_s3::error::BuildError) -> Self {
        Error::builder().kind(ErrorKind::AwsError).message("aws s3 build error").source(e).build()
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::error::Error> for Error {
    fn from(e: gcloud_sdk::error::Error) -> Self {
        Error::builder().kind(ErrorKind::GcpError).message("gcp sdk operation failed").source(e).build()
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::tonic::Status> for Error {
    fn from(e: gcloud_sdk::tonic::Status) -> Self {
        Error::builder().kind(ErrorKind::GcpError).message("gcp sdk operation failed").source(e).build()
    }
}
