use core::fmt;
use std::str::FromStr;

use omnius_core_rocketpack::{RocketMessage, RocketMessageReader, RocketMessageWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniRemotingVersion {
    Unknown,
    V1,
}

impl fmt::Display for OmniRemotingVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &OmniRemotingVersion::V1 => "V1",
            _ => "Unknown",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for OmniRemotingVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "V1" => OmniRemotingVersion::V1,
            _ => OmniRemotingVersion::Unknown,
        };
        Ok(typ)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloMessage {
    pub version: OmniRemotingVersion,
    pub function_id: u32,
}

impl RocketMessage for HelloMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> anyhow::Result<()> {
        writer.put_str(&value.version.to_string());
        writer.put_u32(value.function_id);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let version: OmniRemotingVersion = reader.get_string(1024).map_err(|_| anyhow::anyhow!("invalid version"))?.parse()?;
        let function_id = reader.get_u32().map_err(|_| anyhow::anyhow!("invalid function_id"))?;

        Ok(Self { version, function_id })
    }
}
