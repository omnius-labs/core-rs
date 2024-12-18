use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum RocketMessageError {
    VarintError(VarintError),
    EndOfInput,
    InvalidUtf8,
    TooLarge,
}

impl fmt::Display for RocketMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl std::error::Error for RocketMessageError {}

#[derive(Debug, PartialEq, Eq)]
pub enum VarintError {
    InvalidHeader,
    EndOfInput,
    TooSmallBody,
}

impl fmt::Display for VarintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl std::error::Error for VarintError {}
