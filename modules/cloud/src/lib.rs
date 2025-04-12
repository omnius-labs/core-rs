mod error;
mod prelude;

mod result {
    #[allow(unused)]
    pub type Result<T> = std::result::Result<T, crate::error::Error>;
}

#[cfg(feature = "aws")]
pub mod aws;

#[cfg(feature = "gcp")]
pub mod gcp;

pub use error::*;
pub use result::*;
