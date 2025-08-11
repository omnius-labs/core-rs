use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniRemotingVersion {
    Unknown,
    V1,
}

impl std::fmt::Display for OmniRemotingVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &OmniRemotingVersion::V1 => "V1",
            _ => "Unknown",
        };
        write!(f, "{typ}")
    }
}

impl From<&str> for OmniRemotingVersion {
    fn from(value: &str) -> Self {
        match value {
            "V1" => OmniRemotingVersion::V1,
            _ => OmniRemotingVersion::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloMessage {
    pub version: OmniRemotingVersion,
    pub function_id: u32,
}

impl RocketMessage for HelloMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_str(&value.version.to_string());
        writer.put_u32(value.function_id);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let version = OmniRemotingVersion::from(reader.get_string(1024)?.as_str());
        let function_id = reader.get_u32()?;

        Ok(Self { version, function_id })
    }
}
