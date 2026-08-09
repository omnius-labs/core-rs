mod error;
#[path = "generated/omnikit.rs"]
mod generated;
pub mod model;
mod prelude;
mod result;
pub mod service;

pub use error::*;
pub use result::*;
