pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid header (value {value})")]
    InvalidHeader { value: u8 },

    #[error("End of input")]
    EndOfInput,

    #[error("Too small body (size: {size})")]
    TooSmallBody { size: usize },
}
