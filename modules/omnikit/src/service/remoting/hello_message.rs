use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniRemotingVersion {
    Unknown,
    V1,
}

impl OmniRemotingVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            &Self::V1 => "v1",
            _ => "unknown",
        }
    }
}

impl std::fmt::Display for OmniRemotingVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<T> From<T> for OmniRemotingVersion
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "v1" => OmniRemotingVersion::V1,
            _ => OmniRemotingVersion::Unknown,
        }
    }
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
        encoder.write_string(value.version.as_str())?;

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
                0 => version = OmniRemotingVersion::from(decoder.read_string()?),
                1 => function_id = decoder.read_u32()?,
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self { version, function_id })
    }
}
