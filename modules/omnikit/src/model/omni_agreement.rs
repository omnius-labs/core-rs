use bitflags::bitflags;
use chrono::{DateTime, Utc};
use rand::TryRngCore;
use rand_core::OsRng;

use crate::prelude::*;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct OmniAgreementAlgorithmType: u32 {
        const None = 0;
        const X25519 = 1;
    }
}

impl std::fmt::Display for OmniAgreementAlgorithmType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let typ = match self {
            &OmniAgreementAlgorithmType::X25519 => "X25519",
            _ => "None",
        };
        write!(f, "{typ}")
    }
}

impl From<&str> for OmniAgreementAlgorithmType {
    fn from(value: &str) -> Self {
        match value {
            "X25519" => OmniAgreementAlgorithmType::X25519,
            _ => OmniAgreementAlgorithmType::None,
        }
    }
}

impl OmniAgreementAlgorithmType {
    pub fn as_str(&self) -> &'static str {
        match self {
            &Self::X25519 => "X25519",
            _ => "None",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreement {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl OmniAgreement {
    pub fn new(created_time: DateTime<Utc>, algorithm_type: OmniAgreementAlgorithmType) -> Result<Self> {
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(&mut OsRng.unwrap_err());
        let public_key = x25519_dalek::PublicKey::from(&secret_key);

        let secret_key = secret_key.as_bytes().to_vec();
        let public_key = public_key.as_bytes().to_vec();

        Ok(Self {
            created_time,
            algorithm_type,
            secret_key,
            public_key,
        })
    }

    pub fn gen_agreement_public_key(&self) -> OmniAgreementPublicKey {
        OmniAgreementPublicKey {
            created_time: self.created_time,
            algorithm_type: self.algorithm_type.clone(),
            public_key: self.public_key.clone(),
        }
    }

    pub fn gen_agreement_private_key(&self) -> OmniAgreementPrivateKey {
        OmniAgreementPrivateKey {
            created_time: self.created_time,
            algorithm_type: self.algorithm_type.clone(),
            secret_key: self.secret_key.clone(),
        }
    }

    pub fn gen_secret(private_key: &OmniAgreementPrivateKey, public_key: &OmniAgreementPublicKey) -> Result<Vec<u8>> {
        let secret_key: [u8; 32] = private_key
            .secret_key
            .clone()
            .try_into()
            .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("invalid secret_key"))?;
        let public_key: [u8; 32] = public_key
            .public_key
            .clone()
            .try_into()
            .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("public_key"))?;

        let secret_key = x25519_dalek::StaticSecret::from(secret_key);
        let public_key = x25519_dalek::PublicKey::from(public_key);

        let shared_secret = secret_key.diffie_hellman(&public_key);

        Ok(shared_secret.as_bytes().to_vec())
    }
}

impl RocketMessage for OmniAgreement {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_timestamp64(value.created_time.into());
        writer.put_str(value.algorithm_type.as_str());
        writer.put_bytes(&value.secret_key);
        writer.put_bytes(&value.public_key);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let created_time = reader
            .get_timestamp64()?
            .to_date_time()
            .ok_or_else(|| RocketPackError::new(RocketPackErrorKind::InvalidFormat).with_message("invalid timestamp64"))?;
        let algorithm_type = OmniAgreementAlgorithmType::from(reader.get_string(1024)?.as_str());
        let secret_key = reader.get_bytes(1024)?;
        let public_key = reader.get_bytes(1024)?;

        Ok(Self {
            created_time,
            algorithm_type,
            secret_key,
            public_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreementPublicKey {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub public_key: Vec<u8>,
}

impl RocketMessage for OmniAgreementPublicKey {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_timestamp64(value.created_time.into());
        writer.put_str(value.algorithm_type.as_str());
        writer.put_bytes(&value.public_key);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let created_time = reader
            .get_timestamp64()?
            .to_date_time()
            .ok_or_else(|| RocketPackError::new(RocketPackErrorKind::InvalidFormat).with_message("invalid timestamp64"))?;
        let algorithm_type = OmniAgreementAlgorithmType::from(reader.get_string(1024)?.as_str());
        let public_key = reader.get_bytes(1024)?;

        Ok(Self {
            created_time,
            algorithm_type,
            public_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreementPrivateKey {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
}

impl RocketMessage for OmniAgreementPrivateKey {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_timestamp64(value.created_time.into());
        writer.put_str(value.algorithm_type.as_str());
        writer.put_bytes(&value.secret_key);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let created_time = reader
            .get_timestamp64()?
            .to_date_time()
            .ok_or_else(|| RocketPackError::new(RocketPackErrorKind::InvalidFormat).with_message("invalid timestamp64"))?;
        let algorithm_type = OmniAgreementAlgorithmType::from(reader.get_string(1024)?.as_str());
        let secret_key = reader.get_bytes(1024)?;

        Ok(Self {
            created_time,
            algorithm_type,
            secret_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let agreement1 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::X25519)?;
        let agreement2 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::X25519)?;

        let public_key1 = agreement1.gen_agreement_public_key();
        let private_key1 = agreement1.gen_agreement_private_key();
        let public_key2 = agreement2.gen_agreement_public_key();
        let private_key2 = agreement2.gen_agreement_private_key();

        let secret1 = OmniAgreement::gen_secret(&private_key1, &public_key2)?;
        let secret2 = OmniAgreement::gen_secret(&private_key2, &public_key1)?;

        assert_eq!(secret1, secret2);

        println!("public_key1: {:?}", hex::encode(&public_key1.public_key));
        println!("private_key2: {:?}", hex::encode(&private_key2.secret_key));
        println!("secret2: {:?}", hex::encode(secret2));

        Ok(())
    }
}
