mod empty_rocket_pack;
mod error;
mod prelude;
pub mod primitive;
mod rocket_message;
mod rocket_message_reader;
mod rocket_message_writer;

mod result {
    #[allow(unused)]
    pub type Result<T> = std::result::Result<T, crate::error::Error>;
}

pub use empty_rocket_pack::*;
pub use error::*;
pub use result::*;
pub use rocket_message::*;
pub use rocket_message_reader::*;
pub use rocket_message_writer::*;
