use std::fmt;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

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
        write!(f, "{}:{}", self.typ, hex::encode(&self.value))
    }
}
