use std::fmt;

#[derive(Debug)]
pub enum Error {
    UnexpectedVarintFormat,
    InvalidUtf8,
    TooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl std::error::Error for Error {}
