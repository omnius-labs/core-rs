use std::str::FromStr;

use ed25519_dalek::{
    Signer as _,
    pkcs8::{DecodePrivateKey as _, DecodePublicKey as _, EncodePrivateKey as _, EncodePublicKey as _},
};
use rand::TryRngCore;
use rand_core::OsRng;
use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display, strum::FromRepr)]
pub enum OmniSignType {
    #[strum(serialize = "none")]
    None = 0,
    #[allow(non_camel_case_types)]
    #[strum(serialize = "ed25519_sha3_256_base64url")]
    Ed25519_Sha3_256_Base64Url = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniSigner {
    pub typ: OmniSignType,
    pub name: String,
    pub key: Vec<u8>,
}

impl OmniSigner {
    pub fn new<S: AsRef<str> + ?Sized>(typ: OmniSignType, name: &S) -> Result<Self> {
        match &typ {
            &OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng.unwrap_mut());
                let name = name.as_ref().to_string();
                let key = signing_key.to_pkcs8_der()?.to_bytes().to_vec();
                Ok(Self { typ, name, key })
            }
            _ => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }

    pub fn sign(&self, msg: &[u8]) -> Result<OmniCert> {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(self.key.as_slice())?;

                let typ = self.typ;
                let name = self.name.clone();
                let public_key = signing_key.verifying_key().to_public_key_der()?.into_vec();
                let value = signing_key.sign(msg).to_vec();
                Ok(OmniCert { typ, name, public_key, value })
            }
            _ => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }
}

impl RocketPackStruct for OmniSigner {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(3)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.typ.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_string(&value.name)?;

        encoder.write_u64(2)?;
        encoder.write_bytes(&value.key)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut typ: Option<OmniSignType> = None;
        let mut name: Option<String> = None;
        let mut key: Option<Vec<u8>> = None;

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => typ = Some(OmniSignType::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?),
                1 => name = Some(decoder.read_string()?),
                2 => key = Some(decoder.read_bytes_vec()?),
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            typ: typ.ok_or(RocketPackDecoderError::Other("missing field: typ"))?,
            name: name.ok_or(RocketPackDecoderError::Other("missing field: name"))?,
            key: key.ok_or(RocketPackDecoderError::Other("missing field: key"))?,
        })
    }
}

impl std::fmt::Display for OmniSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(&self.key).map_err(|_| std::fmt::Error)?;
                let public_key = signing_key.verifying_key().to_public_key_der().map_err(|_| std::fmt::Error)?.into_vec();

                let mut hasher = Sha3_256::new();
                hasher.update(public_key);
                let hash = hasher.finalize();

                write!(f, "{}@{}", self.name, OmniBase::encode_by_base64_url(&hash))
            }
            _ => Err(std::fmt::Error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniCert {
    pub typ: OmniSignType,
    pub name: String,
    pub public_key: Vec<u8>,
    pub value: Vec<u8>,
}

impl OmniCert {
    pub fn verify(&self, msg: &[u8]) -> Result<()> {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let public_key = ed25519_dalek::VerifyingKey::from_public_key_der(&self.public_key)?;

                let signature: [u8; ed25519_dalek::SIGNATURE_LENGTH] = self
                    .value
                    .clone()
                    .try_into()
                    .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("invalid public_key"))?;
                let signature = ed25519_dalek::Signature::from_bytes(&signature);

                Ok(public_key
                    .verify_strict(msg, &signature)
                    .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("failed to verify"))?)
            }
            _ => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }
}

impl RocketPackStruct for OmniCert {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(4)?;

        encoder.write_u64(0)?;
        encoder.write_string(value.typ.as_ref())?;

        encoder.write_u64(1)?;
        encoder.write_string(&value.name)?;

        encoder.write_u64(2)?;
        encoder.write_bytes(&value.public_key)?;

        encoder.write_u64(3)?;
        encoder.write_bytes(&value.value)?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut typ: Option<OmniSignType> = None;
        let mut name: Option<String> = None;
        let mut public_key: Option<Vec<u8>> = None;
        let mut value: Option<Vec<u8>> = None;

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => typ = Some(OmniSignType::from_str(&decoder.read_string()?).map_err(|_| RocketPackDecoderError::Other("parse error"))?),
                1 => name = Some(decoder.read_string()?),
                2 => public_key = Some(decoder.read_bytes_vec()?),
                3 => value = Some(decoder.read_bytes_vec()?),
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            typ: typ.ok_or(RocketPackDecoderError::Other("missing field: typ"))?,
            name: name.ok_or(RocketPackDecoderError::Other("missing field: name"))?,
            public_key: public_key.ok_or(RocketPackDecoderError::Other("missing field: public_key"))?,
            value: value.ok_or(RocketPackDecoderError::Other("missing field: value"))?,
        })
    }
}

impl std::fmt::Display for OmniCert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let mut hasher = Sha3_256::new();
                hasher.update(&self.public_key);
                let hash = hasher.finalize();

                write!(f, "{}@{}", self.name, OmniBase::encode_by_base64_url(&hash))
            }
            _ => {
                write!(f, "")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::{OmniSignType, OmniSigner};

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "test_user")?;
        let cert = signer.sign(b"test")?;

        assert_eq!(signer.to_string(), cert.to_string());
        assert!(cert.verify(b"test").is_ok());
        assert!(cert.verify(b"test_err").is_err());

        println!("public_key: {}", hex::encode(cert.public_key));
        println!("value: {}", hex::encode(cert.value));

        Ok(())
    }
}
