use std::str::FromStr;

use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

use super::{OmniHash, OmniHashAlgorithmType};

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
        let typ = match &self.typ {
            OmniHashAlgorithmType::None => "none",
            OmniHashAlgorithmType::Sha3256 => "sha3_256",
        };
        write!(f, "{}:{}", typ, OmniBase::encode_by_base64_url(&self.value))
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
            "sha3_256" => OmniHashAlgorithmType::Sha3256,
            _ => OmniHashAlgorithmType::None,
        };
        let value = OmniBase::decode(value)?;

        Ok(Self { typ, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_roundtrip_test() -> Result<()> {
        let value = OmniHash::compute_hash(OmniHashAlgorithmType::Sha3256, b"test");
        let decoded = OmniHash::import(&value.export()?)?;

        assert_eq!(value, decoded);
        Ok(())
    }
}
