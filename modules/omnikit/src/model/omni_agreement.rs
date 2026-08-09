use chrono::{DateTime, Utc};
use rand::rngs::SysRng;
use rand_core::UnwrapErr;

use crate::prelude::*;

use super::{OmniAgreement, OmniAgreementAlgorithmType, OmniAgreementPrivateKey, OmniAgreementPublicKey};

impl OmniAgreement {
    pub fn new(algorithm_type: OmniAgreementAlgorithmType, created_time: DateTime<Utc>) -> Result<Self> {
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(&mut UnwrapErr(SysRng));
        let public_key = x25519_dalek::PublicKey::from(&secret_key);

        Ok(Self {
            algorithm_type,
            secret_key: secret_key.as_bytes().to_vec(),
            public_key: public_key.as_bytes().to_vec(),
            created_time: created_time.into(),
        })
    }

    pub fn gen_agreement_public_key(&self) -> OmniAgreementPublicKey {
        OmniAgreementPublicKey {
            algorithm_type: self.algorithm_type.clone(),
            public_key: self.public_key.clone(),
            created_time: self.created_time.clone(),
        }
    }

    pub fn gen_agreement_private_key(&self) -> OmniAgreementPrivateKey {
        OmniAgreementPrivateKey {
            algorithm_type: self.algorithm_type.clone(),
            secret_key: self.secret_key.clone(),
            created_time: self.created_time.clone(),
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

        let agreement_decoded = OmniAgreement::import(&agreement1.export()?)?;
        let public_key_decoded = OmniAgreementPublicKey::import(&public_key1.export()?)?;
        let private_key_decoded = OmniAgreementPrivateKey::import(&private_key1.export()?)?;

        assert_eq!(agreement1, agreement_decoded);
        assert_eq!(public_key1, public_key_decoded);
        assert_eq!(private_key1, private_key_decoded);

        let secret1 = OmniAgreement::gen_secret(&private_key1, &public_key2)?;
        let secret2 = OmniAgreement::gen_secret(&private_key2, &public_key1)?;
        assert_eq!(secret1, secret2);

        println!("public_key1: {:?}", hex::encode(&public_key1.public_key));
        println!("private_key2: {:?}", hex::encode(&private_key2.secret_key));
        println!("secret2: {:?}", hex::encode(secret2));

        Ok(())
    }
}
