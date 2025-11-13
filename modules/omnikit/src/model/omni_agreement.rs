use std::str::FromStr;

use chrono::{DateTime, Utc};
use omnius_core_rocketpack::primitive::Timestamp64;
use rand::TryRngCore;
use rand_core::OsRng;

use crate::prelude::*;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display, strum::FromRepr)]
pub enum OmniAgreementAlgorithmType {
    #[strum(serialize = "none")]
    None = 0,
    #[strum(serialize = "x25519")]
    X25519 = 1,
}

impl OmniAgreementAlgorithmType {
    pub const fn bits(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreement {
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub created_time: DateTime<Utc>,
}

impl OmniAgreement {
    pub fn new(algorithm_type: OmniAgreementAlgorithmType, created_time: DateTime<Utc>) -> Result<Self> {
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(&mut OsRng.unwrap_err());
        let public_key = x25519_dalek::PublicKey::from(&secret_key);

        let secret_key = secret_key.as_bytes().to_vec();
        let public_key = public_key.as_bytes().to_vec();

        Ok(Self {
            algorithm_type,
            secret_key,
            public_key,
            created_time,
        })
    }

    pub fn gen_agreement_public_key(&self) -> OmniAgreementPublicKey {
        OmniAgreementPublicKey {
            algorithm_type: self.algorithm_type,
            public_key: self.public_key.clone(),
            created_time: self.created_time,
        }
    }

    pub fn gen_agreement_private_key(&self) -> OmniAgreementPrivateKey {
        OmniAgreementPrivateKey {
            algorithm_type: self.algorithm_type,
            secret_key: self.secret_key.clone(),
            created_time: self.created_time,
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

impl RocketPackStruct for OmniAgreement {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(4)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.algorithm_type.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_bytes(&value.secret_key)?;

        encoder.write_u64(2)?;
        encoder.write_bytes(&value.public_key)?;

        encoder.write_u64(3)?;
        encoder.write_struct(&Timestamp64::from(value.created_time))?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut algorithm_type: OmniAgreementAlgorithmType = OmniAgreementAlgorithmType::None;
        let mut secret_key: Vec<u8> = Vec::new();
        let mut public_key: Vec<u8> = Vec::new();
        let mut created_time: DateTime<Utc> = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => algorithm_type = OmniAgreementAlgorithmType::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?,
                1 => secret_key = decoder.read_bytes_vec()?,
                2 => public_key = decoder.read_bytes_vec()?,
                3 => {
                    created_time = decoder
                        .read_struct::<Timestamp64>()?
                        .to_date_time()
                        .ok_or(RocketPackDecoderError::Other("created_time parse error"))?
                }
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            algorithm_type,
            secret_key,
            public_key,
            created_time,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreementPublicKey {
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub public_key: Vec<u8>,
    pub created_time: DateTime<Utc>,
}

impl RocketPackStruct for OmniAgreementPublicKey {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(3)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.algorithm_type.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_bytes(&value.public_key)?;

        encoder.write_u64(2)?;
        encoder.write_struct(&Timestamp64::from(value.created_time))?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut algorithm_type: OmniAgreementAlgorithmType = OmniAgreementAlgorithmType::None;
        let mut public_key: Vec<u8> = Vec::new();
        let mut created_time: DateTime<Utc> = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => algorithm_type = OmniAgreementAlgorithmType::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?,
                1 => public_key = decoder.read_bytes_vec()?,
                2 => {
                    created_time = decoder
                        .read_struct::<Timestamp64>()?
                        .to_date_time()
                        .ok_or(RocketPackDecoderError::Other("created_time parse error"))?
                }
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            algorithm_type,
            public_key,
            created_time,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniAgreementPrivateKey {
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
    pub created_time: DateTime<Utc>,
}

impl RocketPackStruct for OmniAgreementPrivateKey {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(3)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.algorithm_type.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_bytes(&value.secret_key)?;

        encoder.write_u64(2)?;
        encoder.write_struct(&Timestamp64::from(value.created_time))?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut algorithm_type: OmniAgreementAlgorithmType = OmniAgreementAlgorithmType::None;
        let mut secret_key: Vec<u8> = Vec::new();
        let mut created_time: DateTime<Utc> = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => algorithm_type = OmniAgreementAlgorithmType::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?,
                1 => secret_key = decoder.read_bytes_vec()?,
                2 => {
                    created_time = decoder
                        .read_struct::<Timestamp64>()?
                        .to_date_time()
                        .ok_or(RocketPackDecoderError::Other("created_time parse error"))?
                }
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            algorithm_type,
            secret_key,
            created_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let example_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T01:01:01Z")?.to_utc();
        let agreement1 = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, example_time)?;
        let agreement2 = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, example_time)?;

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
