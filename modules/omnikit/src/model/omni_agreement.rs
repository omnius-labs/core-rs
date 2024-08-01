use bitflags::bitflags;
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct OmniAgreementAlgorithmType: u32 {
        const None = 0;
        const X25519 = 1;
    }
}

pub struct OmniAgreement {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl OmniAgreement {
    pub fn new(created_time: DateTime<Utc>, algorithm_type: OmniAgreementAlgorithmType) -> anyhow::Result<Self> {
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(OsRng);
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

    pub fn gen_secret(private_key: &OmniAgreementPrivateKey, public_key: &OmniAgreementPublicKey) -> anyhow::Result<Vec<u8>> {
        let secret_key: [u8; 32] = private_key
            .secret_key
            .clone()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid secret_key length"))?;
        let public_key: [u8; 32] = public_key
            .public_key
            .clone()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public_key length"))?;

        let secret_key = x25519_dalek::StaticSecret::from(secret_key);
        let public_key = x25519_dalek::PublicKey::from(public_key);

        let shared_secret = secret_key.diffie_hellman(&public_key);

        Ok(shared_secret.as_bytes().to_vec())
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmniAgreementPublicKey {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub public_key: Vec<u8>,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmniAgreementPrivateKey {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub secret_key: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn simple_test() -> TestResult {
        let agreement1 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::X25519).unwrap();
        let agreement2 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::X25519).unwrap();

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
