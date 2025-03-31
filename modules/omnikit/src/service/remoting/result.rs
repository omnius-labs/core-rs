use crate::Result;

pub type CallResult<T, E> = Result<std::result::Result<T, E>>;
