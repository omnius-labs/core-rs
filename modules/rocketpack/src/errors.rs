use std::{error::Error, fmt};

#[derive(Debug)]
pub struct FormatError;

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Format error")
    }
}

impl Error for FormatError {}
