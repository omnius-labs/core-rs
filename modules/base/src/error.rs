use std::backtrace::{Backtrace, BacktraceStatus};

#[allow(unused)]
pub trait OmniError: std::error::Error + std::fmt::Display + std::fmt::Debug {
    type ErrorKind: std::fmt::Display + std::fmt::Debug;

    fn new(kind: Self::ErrorKind) -> Self;
    fn from_error<E: Into<Box<dyn std::error::Error + Send + Sync>>>(source: E, kind: Self::ErrorKind) -> Self;
    fn with_message<S: Into<String>>(self, message: S) -> Self;

    fn kind(&self) -> &Self::ErrorKind;
    fn message(&self) -> Option<&str>;
    fn backtrace(&self) -> &Backtrace;

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Error");

        debug.field("kind", self.kind());

        if let Some(message) = self.message() {
            debug.field("message", &message);
        }

        if let Some(source) = self.source() {
            debug.field("source", &source);
        }

        if self.backtrace().status() == BacktraceStatus::Captured {
            debug.field("backtrace", self.backtrace());
        }

        debug.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum SimpleTestErrorKind {
        Foo,
        Bar,
    }

    impl std::fmt::Display for SimpleTestErrorKind {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SimpleTestErrorKind::Foo => write!(fmt, "foo"),
                SimpleTestErrorKind::Bar => write!(fmt, "bar"),
            }
        }
    }

    struct SimpleTestError {
        kind: SimpleTestErrorKind,
        message: Option<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        backtrace: Backtrace,
    }

    impl OmniError for SimpleTestError {
        type ErrorKind = SimpleTestErrorKind;

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

    impl std::error::Error for SimpleTestError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.source.as_ref().map(|s| &**s as &(dyn std::error::Error + 'static))
        }
    }

    impl std::fmt::Debug for SimpleTestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            OmniError::fmt(self, f)
        }
    }

    impl std::fmt::Display for SimpleTestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match (&self.kind, &self.message) {
                (kind, None) => write!(f, "{kind}"),
                (kind, Some(message)) => write!(f, "{kind}: {message}"),
            }
        }
    }

    #[test]
    #[ignore]
    fn builder_creates_omni_error() {
        let e = SimpleTestError::new(SimpleTestErrorKind::Bar).with_message("message");

        // kind implements Debug via derive
        assert_eq!(format!("{:?}", e.kind()), "Bar");
        assert_eq!(e.message(), Some("message"));
    }

    #[test]
    #[ignore]
    fn debug_print_contains_source_message() {
        let inner = SimpleTestError::new(SimpleTestErrorKind::Foo).with_message("inner message");
        let outer = SimpleTestError::from_error(inner, SimpleTestErrorKind::Bar).with_message("outer message");

        println!("{outer:?}");
    }
}
