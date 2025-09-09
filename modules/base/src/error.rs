use std::backtrace::Backtrace;

#[allow(unused)]
pub trait OmniErrorBuilder<T>
where
    T: OmniError,
{
    type ErrorKind: std::fmt::Display + std::fmt::Debug;

    fn kind(self, kind: Self::ErrorKind) -> Self;
    fn message<S: Into<String>>(self, message: S) -> Self;
    fn source<E: Into<Box<dyn std::error::Error + Send + Sync>>>(self, source: E) -> Self;
    fn backtrace(self) -> Self;

    fn build(self) -> T;
}

#[allow(unused)]
pub trait OmniError: std::error::Error + std::fmt::Display + std::fmt::Debug {
    type ErrorKind: std::fmt::Display + std::fmt::Debug;

    fn kind(&self) -> &Self::ErrorKind;
    fn message(&self) -> Option<&str>;
    fn backtrace(&self) -> Option<&Backtrace>;

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Error");

        debug.field("kind", self.kind());

        if let Some(message) = self.message() {
            debug.field("message", &message);
        }

        if let Some(source) = self.source() {
            debug.field("source", &source);
        }

        if let Some(backtrace) = self.backtrace() {
            debug.field("backtrace", &backtrace.to_string());
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
        backtrace: Option<Backtrace>,
    }

    impl OmniError for SimpleTestError {
        type ErrorKind = SimpleTestErrorKind;

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
            OmniError::fmt(self, f)
        }
    }

    impl SimpleTestError {
        pub fn builder() -> SimpleTestErrorBuilder {
            SimpleTestErrorBuilder {
                inner: Self {
                    kind: SimpleTestErrorKind::Foo,
                    message: None,
                    source: None,
                    backtrace: None,
                },
            }
        }
    }

    struct SimpleTestErrorBuilder {
        inner: SimpleTestError,
    }

    impl OmniErrorBuilder<SimpleTestError> for SimpleTestErrorBuilder {
        type ErrorKind = SimpleTestErrorKind;

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

        fn build(self) -> SimpleTestError {
            self.inner
        }
    }

    #[test]
    #[ignore]
    fn builder_creates_omni_error() {
        let e = SimpleTestError::builder().kind(SimpleTestErrorKind::Bar).message("builder message").backtrace().build();

        // kind implements Debug via derive
        assert_eq!(format!("{:?}", e.kind()), "Bar");
        assert_eq!(e.message(), Some("builder message"));
        assert!(e.backtrace().is_some());
    }

    #[test]
    #[ignore]
    fn debug_print_contains_source_message() {
        let inner = SimpleTestError::builder().kind(SimpleTestErrorKind::Foo).message("inner message").backtrace().build();

        let outer = SimpleTestError::builder().kind(SimpleTestErrorKind::Bar).message("outer message").source(inner).build();

        println!("{outer:?}");
    }
}
