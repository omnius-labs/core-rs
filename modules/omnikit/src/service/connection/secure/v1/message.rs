use std::{
    fmt::{self, Display},
    str::FromStr,
};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use omnius_core_rocketpack::{RocketMessage, RocketMessageReader, RocketMessageWriter};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct AuthType: u32 {
        const None = 0;
        const Sign = 1;
    }
}

impl Display for AuthType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &AuthType::Sign => "Sign",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for AuthType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "Sign" => AuthType::Sign,
            _ => AuthType::None,
        };
        Ok(typ)
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyExchangeAlgorithmType: u32 {
        const None = 0;
        const X25519 = 1;
    }
}

impl Display for KeyExchangeAlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &KeyExchangeAlgorithmType::X25519 => "X25519",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for KeyExchangeAlgorithmType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "X25519" => KeyExchangeAlgorithmType::X25519,
            _ => KeyExchangeAlgorithmType::None,
        };
        Ok(typ)
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyDerivationAlgorithmType: u32 {
        const None = 0;
        const Hkdf = 1;
    }
}

impl Display for KeyDerivationAlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &KeyDerivationAlgorithmType::Hkdf => "Hkdf",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for KeyDerivationAlgorithmType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "Hkdf" => KeyDerivationAlgorithmType::Hkdf,
            _ => KeyDerivationAlgorithmType::None,
        };
        Ok(typ)
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct CipherAlgorithmType: u32 {
        const None = 0;
        const Aes256Gcm = 1;
    }
}

impl Display for CipherAlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &CipherAlgorithmType::Aes256Gcm => "Aes256Gcm",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for CipherAlgorithmType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "Aes256Gcm" => CipherAlgorithmType::Aes256Gcm,
            _ => CipherAlgorithmType::None,
        };
        Ok(typ)
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct HashAlgorithmType: u32 {
        const None = 0;
        const Sha3_256 = 1;
    }
}

impl Display for HashAlgorithmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &HashAlgorithmType::Sha3_256 => "Sha3_256",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for HashAlgorithmType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "Sha3_256" => HashAlgorithmType::Sha3_256,
            _ => HashAlgorithmType::None,
        };
        Ok(typ)
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
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> anyhow::Result<()> {
        writer.put_bytes(&value.session_id);
        writer.put_str(value.auth_type.to_string().as_str());
        writer.put_str(value.key_exchange_algorithm_type.to_string().as_str());
        writer.put_str(value.key_derivation_algorithm_type.to_string().as_str());
        writer.put_str(value.cipher_algorithm_type.to_string().as_str());
        writer.put_str(value.hash_algorithm_type.to_string().as_str());

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let session_id = reader
            .get_bytes(1024)
            .map_err(|_| anyhow::anyhow!("invalid session_id"))?;
        let auth_type: AuthType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid auth_type"))?
            .parse()?;
        let key_exchange_algorithm_type: KeyExchangeAlgorithmType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid key_exchange_algorithm_type"))?
            .parse()?;
        let key_derivation_algorithm_type: KeyDerivationAlgorithmType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid key_derivation_algorithm_type"))?
            .parse()?;
        let cipher_algorithm_type: CipherAlgorithmType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid cipher_algorithm_type"))?
            .parse()?;
        let hash_algorithm_type: HashAlgorithmType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid hash_algorithm_type"))?
            .parse()?;

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
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL, Engine};
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
