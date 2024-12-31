use std::fmt;
use std::str::FromStr;

use bitflags::bitflags;
use ed25519_dalek::pkcs8::{
    DecodePrivateKey as _, DecodePublicKey as _, EncodePrivateKey as _, EncodePublicKey as _,
};
use ed25519_dalek::Signer as _;
use omnius_core_rocketpack::{RocketMessage, RocketMessageReader, RocketMessageWriter};
use rand_core::OsRng;
use sha3::{Digest, Sha3_256};

use crate::service::converter::OmniBase;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OmniSignType: u32 {
        const None = 0;
        const Ed25519_Sha3_256_Base64Url = 1;
    }
}

impl fmt::Display for OmniSignType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let typ = match self {
            &OmniSignType::Ed25519_Sha3_256_Base64Url => "Ed25519_Sha3_256_Base64Url",
            _ => "None",
        };
        write!(f, "{}", typ)
    }
}

impl FromStr for OmniSignType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let typ = match s {
            "Ed25519_Sha3_256_Base64Url" => OmniSignType::Ed25519_Sha3_256_Base64Url,
            _ => OmniSignType::None,
        };
        Ok(typ)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniSigner {
    pub typ: OmniSignType,
    pub name: String,
    pub key: Vec<u8>,
}

impl OmniSigner {
    pub fn new<S: AsRef<str> + ?Sized>(typ: OmniSignType, name: &S) -> anyhow::Result<Self> {
        match &typ {
            &OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
                let name = name.as_ref().to_string();
                let key = signing_key.to_pkcs8_der()?.to_bytes().to_vec();
                Ok(Self { typ, name, key })
            }
            _ => anyhow::bail!("Unsupported sign type"),
        }
    }

    pub fn sign(&self, msg: &[u8]) -> anyhow::Result<OmniCert> {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(self.key.as_slice())?;

                let typ = self.typ.clone();
                let name = self.name.clone();
                let public_key = signing_key.verifying_key().to_public_key_der()?.into_vec();
                let value = signing_key.sign(msg).to_vec();
                Ok(OmniCert {
                    typ,
                    name,
                    public_key,
                    value,
                })
            }
            _ => anyhow::bail!("Unsupported sign type"),
        }
    }
}

impl RocketMessage for OmniSigner {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> anyhow::Result<()> {
        writer.put_str(value.typ.to_string().as_str());
        writer.put_str(&value.name);
        writer.put_bytes(&value.key);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let typ: OmniSignType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid typ"))?
            .parse()?;
        let name = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid name"))?
            .parse()?;
        let key = reader
            .get_bytes(1024)
            .map_err(|_| anyhow::anyhow!("invalid key"))?;

        Ok(Self { typ, name, key })
    }
}

impl fmt::Display for OmniSigner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key =
                    ed25519_dalek::SigningKey::from_pkcs8_der(&self.key).map_err(|_| fmt::Error)?;
                let public_key = signing_key
                    .verifying_key()
                    .to_public_key_der()
                    .map_err(|_| fmt::Error)?
                    .into_vec();

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
    pub fn verify(&self, msg: &[u8]) -> anyhow::Result<()> {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let public_key =
                    ed25519_dalek::VerifyingKey::from_public_key_der(&self.public_key)?;

                let signature: [u8; ed25519_dalek::SIGNATURE_LENGTH] = self
                    .value
                    .clone()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
                let signature = ed25519_dalek::Signature::from_bytes(&signature);

                Ok(public_key.verify_strict(msg, &signature)?)
            }
            _ => anyhow::bail!("Unsupported sign type"),
        }
    }
}

impl RocketMessage for OmniCert {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> anyhow::Result<()> {
        writer.put_str(value.typ.to_string().as_str());
        writer.put_str(&value.name);
        writer.put_bytes(&value.public_key);
        writer.put_bytes(&value.value);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let typ: OmniSignType = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid typ"))?
            .parse()?;
        let name = reader
            .get_string(1024)
            .map_err(|_| anyhow::anyhow!("invalid name"))?
            .parse()?;
        let public_key = reader
            .get_bytes(1024)
            .map_err(|_| anyhow::anyhow!("invalid public_key"))?;
        let value = reader
            .get_bytes(1024)
            .map_err(|_| anyhow::anyhow!("invalid value"))?;

        Ok(Self {
            typ,
            name,
            public_key,
            value,
        })
    }
}

impl fmt::Display for OmniCert {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
