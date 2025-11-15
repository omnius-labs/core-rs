use serde::{Deserialize, Serialize};

use crate::RocketPackStruct;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyRocketMessage;

impl RocketPackStruct for EmptyRocketMessage {
    fn pack(_encoder: &mut impl crate::RocketPackEncoder, _value: &Self) -> std::result::Result<(), crate::RocketPackEncoderError> {
        Ok(())
    }

    fn unpack(_decoder: &mut impl crate::RocketPackDecoder) -> std::result::Result<Self, crate::RocketPackDecoderError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}
