use std::fmt;

use bitflags::bitflags;
use ed25519_dalek::pkcs8::{DecodePrivateKey as _, DecodePublicKey as _, EncodePrivateKey as _, EncodePublicKey as _};
use ed25519_dalek::Signer as _;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::converter::OmniBase;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct OmniSignType: u32 {
        const None = 0;
        const Ed25519_Sha3_256_Base64Url = 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmniSigner {
    pub typ: OmniSignType,
    pub name: String,
    pub key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmniCert {
    pub typ: OmniSignType,
    pub name: String,
    pub public_key: Vec<u8>,
    pub value: Vec<u8>,
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

impl fmt::Display for OmniSigner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(&self.key).map_err(|_| fmt::Error)?;
                let public_key = signing_key.verifying_key().to_public_key_der().map_err(|_| fmt::Error)?.into_vec();

                let mut hasher = Sha3_256::new();
                hasher.update(public_key);
                let hash = hasher.finalize();

                write!(f, "{}@{}", self.name, OmniBase::encode_by_base64_url(&hash))
            }
            _ => Err(std::fmt::Error),
        }
    }
}

impl OmniCert {
    pub fn verify(&self, msg: &[u8]) -> anyhow::Result<()> {
        match self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                let public_key = ed25519_dalek::VerifyingKey::from_public_key_der(&self.public_key)?;

                let signature: [u8; ed25519_dalek::SIGNATURE_LENGTH] =
                    self.value.clone().try_into().map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
                let signature = ed25519_dalek::Signature::from_bytes(&signature);

                Ok(public_key.verify_strict(msg, &signature)?)
            }
            _ => anyhow::bail!("Unsupported sign type"),
        }
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
