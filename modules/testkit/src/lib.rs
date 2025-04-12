pub mod containers;
mod error;
mod prelude;

mod result {
    #[allow(unused)]
    pub type Result<T> = std::result::Result<T, crate::error::Error>;
}

pub use error::*;
pub use result::*;
