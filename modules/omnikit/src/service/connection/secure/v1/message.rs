use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct AuthType: u32 {
        const None = 0;
        const Sign = 1;
    }
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &AuthType::Sign => "Sign",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl std::str::FromStr for AuthType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Sign" => Ok(AuthType::Sign),
            _ => Ok(AuthType::None),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyExchangeAlgorithmType: u32 {
        const None = 0;
        const X25519 = 1;
    }
}

impl std::fmt::Display for KeyExchangeAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &KeyExchangeAlgorithmType::X25519 => "X25519",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl std::str::FromStr for KeyExchangeAlgorithmType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "X25519" => Ok(KeyExchangeAlgorithmType::X25519),
            _ => Ok(KeyExchangeAlgorithmType::None),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyDerivationAlgorithmType: u32 {
        const None = 0;
        const Hkdf = 1;
    }
}

impl std::fmt::Display for KeyDerivationAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &KeyDerivationAlgorithmType::Hkdf => "Hkdf",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl std::str::FromStr for KeyDerivationAlgorithmType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Hkdf" => Ok(KeyDerivationAlgorithmType::Hkdf),
            _ => Ok(KeyDerivationAlgorithmType::None),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct CipherAlgorithmType: u32 {
        const None = 0;
        const Aes256Gcm = 1;
    }
}

impl std::fmt::Display for CipherAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &CipherAlgorithmType::Aes256Gcm => "Aes256Gcm",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl std::str::FromStr for CipherAlgorithmType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Aes256Gcm" => Ok(CipherAlgorithmType::Aes256Gcm),
            _ => Ok(CipherAlgorithmType::None),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct HashAlgorithmType: u32 {
        const None = 0;
        const Sha3_256 = 1;
    }
}

impl std::fmt::Display for HashAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &HashAlgorithmType::Sha3_256 => "Sha3_256",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl std::str::FromStr for HashAlgorithmType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Sha3_256" => Ok(HashAlgorithmType::Sha3_256),
            _ => Ok(HashAlgorithmType::None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProfileMessage {
    pub session_id: Vec<u8>,
    pub auth_type: AuthType,
    pub key_exchange_algorithm_type: KeyExchangeAlgorithmType,
    pub key_derivation_algorithm_type: KeyDerivationAlgorithmType,
    pub cipher_algorithm_type: CipherAlgorithmType,
    pub hash_algorithm_type: HashAlgorithmType,
}

impl RocketMessage for ProfileMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_bytes(&value.session_id);
        writer.put_u32(value.auth_type.bits());
        writer.put_u32(value.key_exchange_algorithm_type.bits());
        writer.put_u32(value.key_derivation_algorithm_type.bits());
        writer.put_u32(value.cipher_algorithm_type.bits());
        writer.put_u32(value.hash_algorithm_type.bits());

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let session_id = reader.get_bytes(1024)?;
        let auth_type = AuthType::from_bits_truncate(reader.get_u32()?);
        let key_exchange_algorithm_type = KeyExchangeAlgorithmType::from_bits_truncate(reader.get_u32()?);
        let key_derivation_algorithm_type = KeyDerivationAlgorithmType::from_bits_truncate(reader.get_u32()?);
        let cipher_algorithm_type = CipherAlgorithmType::from_bits_truncate(reader.get_u32()?);
        let hash_algorithm_type = HashAlgorithmType::from_bits_truncate(reader.get_u32()?);

        Ok(Self {
            session_id,
            auth_type,
            key_exchange_algorithm_type,
            key_derivation_algorithm_type,
            cipher_algorithm_type,
            hash_algorithm_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL};
    use testresult::TestResult;

    use super::*;

    #[ignore]
    #[test]
    fn simple_test() -> TestResult {
        let p = ProfileMessage {
            session_id: vec![1, 2, 3, 4],
            auth_type: AuthType::Sign,
            key_exchange_algorithm_type: KeyExchangeAlgorithmType::X25519,
            key_derivation_algorithm_type: KeyDerivationAlgorithmType::Hkdf,
            cipher_algorithm_type: CipherAlgorithmType::Aes256Gcm,
            hash_algorithm_type: HashAlgorithmType::Sha3_256,
        };

        let b = p.export()?;
        let p2 = ProfileMessage::import(&mut b.clone())?;

        assert_eq!(p, p2);

        let v = b.to_vec();
        println!("{:?}", BASE64_URL.encode(v.as_slice()));

        Ok(())
    }
}
