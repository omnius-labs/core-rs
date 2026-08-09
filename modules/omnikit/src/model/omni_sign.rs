use ed25519_dalek::{
    Signer as _,
    pkcs8::{DecodePrivateKey as _, DecodePublicKey as _, EncodePrivateKey as _, EncodePublicKey as _},
};
use rand::rngs::SysRng;
use rand_core::UnwrapErr;
use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

use super::{OmniCert, OmniSignType, OmniSigner};

impl OmniSigner {
    pub fn new<S: AsRef<str> + ?Sized>(typ: OmniSignType, name: &S) -> Result<Self> {
        match &typ {
            OmniSignType::Ed25519Sha3256Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
                let name = name.as_ref().to_string();
                let key = signing_key.to_pkcs8_der()?.to_bytes().to_vec();
                Ok(Self { typ, name, key })
            }
            OmniSignType::None => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }

    pub fn sign(&self, msg: &[u8]) -> Result<OmniCert> {
        match &self.typ {
            OmniSignType::Ed25519Sha3256Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(self.key.as_slice())?;

                let typ = self.typ.clone();
                let name = self.name.clone();
                let public_key = signing_key.verifying_key().to_public_key_der()?.into_vec();
                let value = signing_key.sign(msg).to_vec();
                Ok(OmniCert { typ, name, public_key, value })
            }
            OmniSignType::None => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }
}

impl std::fmt::Display for OmniSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.typ {
            OmniSignType::Ed25519Sha3256Base64Url => {
                let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(&self.key).map_err(|_| std::fmt::Error)?;
                let public_key = signing_key.verifying_key().to_public_key_der().map_err(|_| std::fmt::Error)?.into_vec();

                let mut hasher = Sha3_256::new();
                hasher.update(public_key);
                let hash = hasher.finalize();

                write!(f, "{}@{}", self.name, OmniBase::encode_by_base64_url(&hash))
            }
            OmniSignType::None => Err(std::fmt::Error),
        }
    }
}

impl OmniCert {
    pub fn verify(&self, msg: &[u8]) -> Result<()> {
        match &self.typ {
            OmniSignType::Ed25519Sha3256Base64Url => {
                let public_key = ed25519_dalek::VerifyingKey::from_public_key_der(&self.public_key)?;

                let signature: [u8; ed25519_dalek::SIGNATURE_LENGTH] = self
                    .value
                    .clone()
                    .try_into()
                    .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("invalid public_key"))?;
                let signature = ed25519_dalek::Signature::from_bytes(&signature);

                public_key
                    .verify_strict(msg, &signature)
                    .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("failed to verify"))?;
                Ok(())
            }
            OmniSignType::None => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }
}

impl std::fmt::Display for OmniCert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.typ {
            OmniSignType::Ed25519Sha3256Base64Url => {
                let mut hasher = Sha3_256::new();
                hasher.update(&self.public_key);
                let hash = hasher.finalize();

                write!(f, "{}@{}", self.name, OmniBase::encode_by_base64_url(&hash))
            }
            OmniSignType::None => f.write_str(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519Sha3256Base64Url, "test_user")?;
        let cert = signer.sign(b"test")?;

        let signer_decoded = OmniSigner::import(&signer.export()?)?;
        let cert_decoded = OmniCert::import(&cert.export()?)?;

        assert_eq!(signer, signer_decoded);
        assert_eq!(cert, cert_decoded);
        assert_eq!(signer.to_string(), cert.to_string());
        assert!(cert.verify(b"test").is_ok());
        assert!(cert.verify(b"test_err").is_err());

        println!("public_key: {}", hex::encode(cert.public_key));
        println!("value: {}", hex::encode(cert.value));

        Ok(())
    }
}
