use std::str::FromStr;

use bitflags::bitflags;
use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct OmniHashAlgorithmType: u32 {
        const None = 0;
        const Sha3_256 = 1;
    }
}

impl std::fmt::Display for OmniHashAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &OmniHashAlgorithmType::Sha3_256 => "Sha3_256",
            _ => "None",
        };

        write!(f, "{typ}",)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OmniHash {
    pub typ: OmniHashAlgorithmType,
    pub value: Vec<u8>,
}

impl OmniHash {
    pub fn compute_hash(typ: OmniHashAlgorithmType, bytes: &[u8]) -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(bytes);
        let value = hasher.finalize().to_vec();
        Self { typ, value }
    }
}

impl std::fmt::Display for OmniHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.typ, OmniBase::encode_by_base64_url(&self.value))
    }
}

impl Default for OmniHash {
    fn default() -> Self {
        Self {
            typ: OmniHashAlgorithmType::None,
            value: Vec::new(),
        }
    }
}

impl FromStr for OmniHash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut iter = s.split(':');

        let typ = iter
            .next()
            .ok_or_else(|| Error::builder().kind(ErrorKind::InvalidFormat).message("type not found").build())?;
        let value = iter
            .next()
            .ok_or_else(|| Error::builder().kind(ErrorKind::InvalidFormat).message("value not found").build())?;

        let typ = match typ {
            "Sha3_256" => OmniHashAlgorithmType::Sha3_256,
            _ => OmniHashAlgorithmType::None,
        };

        let value = OmniBase::decode(value)?;

        Ok(OmniHash { typ, value })
    }
}

impl RocketMessage for OmniHash {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_u32(value.typ.bits());
        writer.put_bytes(&value.value);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let typ = OmniHashAlgorithmType::from_bits(reader.get_u32()?).ok_or_else(|| {
            RocketPackError::builder()
                .kind(RocketPackErrorKind::InvalidFormat)
                .message("any unknown bits are set")
                .build()
        })?;
        let value = reader.get_bytes(1024)?;

        Ok(Self { typ, value })
    }
}
