use std::str::FromStr;

use strum;

use crate::prelude::*;

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display)]
pub enum OmniRemotingVersion {
    #[strum(serialize = "unknown")]
    Unknown,
    #[strum(serialize = "v1")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloMessage {
    pub version: OmniRemotingVersion,
    pub function_id: u32,
}

impl RocketPackStruct for HelloMessage {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(2)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.version.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_u32(value.function_id)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut version: OmniRemotingVersion = OmniRemotingVersion::Unknown;
        let mut function_id: u32 = 0;

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => version = OmniRemotingVersion::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?,
                1 => function_id = decoder.read_u32()?,
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self { version, function_id })
    }
}
