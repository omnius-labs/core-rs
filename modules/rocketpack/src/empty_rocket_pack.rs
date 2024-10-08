use serde::{Deserialize, Serialize};

use crate::{RocketMessage, RocketMessageReader, RocketMessageWriter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyRocketMessage;

impl RocketMessage for EmptyRocketMessage {
    fn pack(_writer: &mut RocketMessageWriter, _value: &Self, _depth: u32) {}

    fn unpack(_reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }
}
