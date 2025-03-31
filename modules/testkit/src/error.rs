pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    TestcontainersError,
    InvalidFormat,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::TestcontainersError => write!(fmt, "testcontainers error"),
            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
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

impl From<std::convert::Infallible> for Error {
    fn from(e: std::convert::Infallible) -> Self {
        Error::new(ErrorKind::InvalidFormat).message("convert failed").source(e)
    }
}

impl From<testcontainers::TestcontainersError> for Error {
    fn from(e: testcontainers::TestcontainersError) -> Self {
        Error::new(ErrorKind::TestcontainersError).message("testcontainers error").source(e)
    }
}
