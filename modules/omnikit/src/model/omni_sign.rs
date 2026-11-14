use ed25519_dalek::{
    Signer as _,
    pkcs8::{DecodePrivateKey as _, DecodePublicKey as _, EncodePrivateKey as _, EncodePublicKey as _},
};
use rand::rngs::SysRng;
use rand_core::UnwrapErr;
use sha3::{Digest, Sha3_256};

use crate::{prelude::*, service::converter::OmniBase};

use crate::generated::omni_sign::*;

impl OmniSigner {
    /// A signer or certificate name is a handle of 0–32 bytes of `[A-Za-z0-9._-]`.
    /// The empty name is allowed because NodeProfile verifies a certificate with an empty name.
    /// The wire bounds enforce the length; the inherent creation, import, export, signing, and display APIs enforce the character set.
    /// The explicit `RocketPackStruct` import and export path is a low-level codec and enforces only the wire length.
    pub fn validate_name(name: &str) -> Result<()> {
        let valid = name.len() <= MAX_NAME_LENGTH as usize && name.bytes().all(|value| value.is_ascii_alphanumeric() || matches!(value, b'.' | b'_' | b'-'));
        if !valid {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("name must be 0-32 bytes of [A-Za-z0-9._-]"));
        }
        Ok(())
    }

    pub fn new<S: AsRef<str> + ?Sized>(typ: OmniSignType, name: &S) -> Result<Self> {
        match &typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                Self::validate_name(name.as_ref())?;
                let signing_key = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
                let name = name.as_ref().to_string();
                let key = signing_key.to_pkcs8_der()?.to_bytes().to_vec();
                Ok(Self { typ, name, key })
            }
            OmniSignType::None => Err(Error::new(ErrorKind::UnsupportedType).with_message("sign type")),
        }
    }

    pub fn import(bytes: &[u8]) -> Result<Self> {
        let signer = <Self as RocketPackStruct>::import(bytes)?;
        Self::validate_name(&signer.name)?;
        Ok(signer)
    }

    pub fn export(&self) -> Result<Vec<u8>> {
        Self::validate_name(&self.name)?;
        Ok(<Self as RocketPackStruct>::export(self)?)
    }

    pub fn sign(&self, msg: &[u8]) -> Result<OmniCert> {
        Self::validate_name(&self.name)?;
        match &self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
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
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                Self::validate_name(&self.name).map_err(|_| std::fmt::Error)?;
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
    pub fn validate_name(name: &str) -> Result<()> {
        OmniSigner::validate_name(name)
    }

    pub fn import(bytes: &[u8]) -> Result<Self> {
        let cert = <Self as RocketPackStruct>::import(bytes)?;
        Self::validate_name(&cert.name)?;
        Ok(cert)
    }

    pub fn export(&self) -> Result<Vec<u8>> {
        Self::validate_name(&self.name)?;
        Ok(<Self as RocketPackStruct>::export(self)?)
    }

    pub fn verify(&self, msg: &[u8]) -> Result<()> {
        Self::validate_name(&self.name)?;
        match &self.typ {
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
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
            OmniSignType::Ed25519_Sha3_256_Base64Url => {
                Self::validate_name(&self.name).map_err(|_| std::fmt::Error)?;
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
    use omnius_core_rocketpack::RocketPackStruct;
    use testresult::TestResult;

    use super::*;

    #[test]
    fn signer_name_accepts_handles() -> TestResult {
        for name in ["", "a", "Anonymous", "axus-node", "test_user", "A0._-"] {
            let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, name)?;
            assert_eq!(signer.name, name);
        }
        let max = "a".repeat(32);
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, &max)?;
        assert_eq!(signer.name, max);
        Ok(())
    }

    #[test]
    fn signer_name_rejects_invalid_handles() -> TestResult {
        for name in [" ", "with space", "with@at", "with/slash", "日本語"] {
            assert!(OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, name).is_err(), "{name}");
        }
        let long = "a".repeat(33);
        assert!(OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, &long).is_err());
        Ok(())
    }

    #[test]
    fn import_rejects_an_invalid_name() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "valid")?;
        let mut invalid = signer.clone();
        invalid.name = "bad name".to_string();
        assert!(invalid.export().is_err());
        assert!(invalid.sign(b"test").is_err());
        let bytes = <OmniSigner as RocketPackStruct>::export(&invalid)?;
        assert!(OmniSigner::import(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn wire_rejects_a_name_outside_the_bounds() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "valid")?;
        let mut long = signer.clone();
        long.name = "a".repeat(33);
        assert!(<OmniSigner as RocketPackStruct>::export(&long).is_err());

        let mut cert = signer.sign(b"test")?;
        cert.name = "a".repeat(33);
        assert!(<OmniCert as RocketPackStruct>::export(&cert).is_err());
        Ok(())
    }

    #[test]
    fn empty_name_round_trips_for_signer_and_cert() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "")?;
        let decoded = OmniSigner::import(&signer.export()?)?;
        assert_eq!(decoded.name, "");

        let cert = signer.sign(b"test")?;
        let decoded_cert = OmniCert::import(&cert.export()?)?;
        assert_eq!(decoded_cert.name, "");
        assert!(decoded_cert.verify(b"test").is_ok());
        Ok(())
    }

    #[test]
    fn cert_with_an_invalid_name_is_rejected() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "valid")?;
        let mut cert = signer.sign(b"test")?;
        cert.name = "bad name".to_string();
        assert!(cert.verify(b"test").is_err());
        assert!(cert.export().is_err());

        let bytes = <OmniCert as RocketPackStruct>::export(&cert)?;
        assert!(OmniCert::import(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn cert_with_an_empty_name_still_verifies() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "valid")?;
        let mut cert = signer.sign(b"test")?;
        cert.name.clear();
        assert!(cert.verify(b"test").is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let signer = OmniSigner::new(OmniSignType::Ed25519_Sha3_256_Base64Url, "test_user")?;
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
