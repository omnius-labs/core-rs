use std::{fmt, str::FromStr};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::converter::OmniBase;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct OmniHashAlgorithmType: u32 {
        const None = 0;
        const Sha3_256 = 1;
    }
}

impl fmt::Display for OmniHashAlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &OmniHashAlgorithmType::Sha3_256 => "sha3-256",
            _ => "none",
        };

        write!(f, "{}", typ)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OmniHash {
    pub typ: OmniHashAlgorithmType,
    pub value: Vec<u8>,
}

impl fmt::Display for OmniHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.typ, OmniBase::encode_by_base64_url(&self.value))
    }
}

impl FromStr for OmniHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split(':');

        let typ = iter.next().ok_or_else(|| anyhow::anyhow!("invalid omni hash"))?;
        let value = iter.next().ok_or_else(|| anyhow::anyhow!("invalid omni hash"))?;

        let typ = match typ {
            "sha3-256" => OmniHashAlgorithmType::Sha3_256,
            _ => OmniHashAlgorithmType::None,
        };

        let value = OmniBase::decode(value)?;

        Ok(OmniHash { typ, value })
    }
}
