mod error;
mod prelude;

mod result {
    #[allow(unused)]
    pub type Result<T> = std::result::Result<T, crate::error::Error>;
}

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use error::*;
pub use result::*;
