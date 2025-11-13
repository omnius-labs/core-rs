use std::str::FromStr;

use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::AsRefStr, strum::Display, strum::FromRepr)]
pub enum OmniHashAlgorithmType {
    #[strum(serialize = "none")]
    None = 0,
    #[strum(serialize = "sha3_256")]
    Sha3_256 = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OmniHash {
    pub typ: OmniHashAlgorithmType,
    pub value: Vec<u8>,
}

impl OmniHash {
    pub fn compute_hash<V>(typ: OmniHashAlgorithmType, bytes: V) -> Self
    where
        V: AsRef<[u8]>,
    {
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

        let typ = iter.next().ok_or_else(|| Error::new(ErrorKind::InvalidFormat).with_message("type not found"))?;
        let value = iter.next().ok_or_else(|| Error::new(ErrorKind::InvalidFormat).with_message("value not found"))?;

        let typ = match typ {
            "sha3_256" => OmniHashAlgorithmType::Sha3_256,
            _ => OmniHashAlgorithmType::None,
        };

        let value = OmniBase::decode(value)?;

        Ok(OmniHash { typ, value })
    }
}

impl RocketPackStruct for OmniHash {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(2)?;

        encoder.write_u64(0)?;
        encoder.write_u32(value.typ as u32)?;

        encoder.write_u64(1)?;
        encoder.write_bytes(&value.value)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut typ: OmniHashAlgorithmType = OmniHashAlgorithmType::None;
        let mut value: Vec<u8> = Vec::new();

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => typ = OmniHashAlgorithmType::from_repr(decoder.read_u32()?).ok_or(RocketPackDecoderError::Other("parse error"))?,
                1 => value = decoder.read_bytes_vec()?,
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self { typ, value })
    }
}
