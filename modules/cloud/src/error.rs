#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
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
            ErrorKind::IoError => write!(fmt, "I/O error"),
            ErrorKind::TimeError => write!(fmt, "time conversion error"),

            ErrorKind::AwsError => write!(fmt, "AWS error"),
            ErrorKind::GcpError => write!(fmt, "GCP error"),

            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
            ErrorKind::NotFound => write!(fmt, "not found"),
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
        let mut debug = fmt.debug_struct("Error");

        debug.field("kind", &self.kind);

        if let Some(message) = &self.message {
            debug.field("message", message);
        }

        if let Some(source) = &self.source {
            debug.field("source", source);
        }

        debug.finish()
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

impl From<std::convert::Infallible> for Error {
    fn from(e: std::convert::Infallible) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("convert failed").source(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(ErrorKind::IoError).message("I/O operation failed").source(e)
    }
}

impl From<chrono::OutOfRangeError> for Error {
    fn from(e: chrono::OutOfRangeError) -> Self {
        Error::new(ErrorKind::TimeError).message("Time conversion failed").source(e)
    }
}

#[cfg(feature = "aws")]
impl<E, R> From<aws_smithy_runtime_api::client::result::SdkError<E, R>> for Error
where
    E: std::error::Error + Send + Sync + 'static,
    R: std::fmt::Debug + Send + Sync + 'static,
{
    fn from(e: aws_smithy_runtime_api::client::result::SdkError<E, R>) -> Self {
        Error::new(ErrorKind::AwsError).message("AWS SDK operation failed").source(e)
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::primitives::ByteStreamError> for Error {
    fn from(e: aws_sdk_s3::primitives::ByteStreamError) -> Self {
        Error::new(ErrorKind::AwsError).message("AWS S3 byte stream error").source(e)
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::presigning::PresigningConfigError> for Error {
    fn from(e: aws_sdk_s3::presigning::PresigningConfigError) -> Self {
        Error::new(ErrorKind::AwsError).message("AWS S3 presigning config error").source(e)
    }
}

#[cfg(feature = "aws")]
impl From<aws_sdk_s3::error::BuildError> for Error {
    fn from(e: aws_sdk_s3::error::BuildError) -> Self {
        Error::new(ErrorKind::AwsError).message("AWS S3 build error").source(e)
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::error::Error> for Error {
    fn from(e: gcloud_sdk::error::Error) -> Self {
        Error::new(ErrorKind::GcpError).message("GCP operation failed").source(e)
    }
}

#[cfg(feature = "gcp")]
impl From<gcloud_sdk::tonic::Status> for Error {
    fn from(e: gcloud_sdk::tonic::Status) -> Self {
        Error::new(ErrorKind::GcpError).message("GCP operation failed").source(e)
    }
}
