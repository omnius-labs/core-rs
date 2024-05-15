use bitflags::bitflags;
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct OmniAgreementAlgorithmType: u32 {
        const None = 0;
        const EcDhP256 = 1;
    }
}

pub struct OmniAgreement {
    pub created_time: DateTime<Utc>,
    pub algorithm_type: OmniAgreementAlgorithmType,
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

impl OmniAgreement {
    pub fn new(created_time: DateTime<Utc>, algorithm_type: OmniAgreementAlgorithmType) -> anyhow::Result<Self> {
        let scalar = p256::NonZeroScalar::random(&mut OsRng);
        let public_key = p256::PublicKey::from_secret_scalar(&scalar).to_sec1_bytes().to_vec();
        let secret_key = p256::SecretKey::from(&scalar).to_sec1_der()?.to_vec();

        Ok(Self {
            created_time,
            algorithm_type,
            public_key,
            secret_key,
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
        let public_key = p256::PublicKey::from_sec1_bytes(&public_key.public_key).or_else(|_| anyhow::bail!("Invalid public key"))?;
        let secret_key = p256::SecretKey::from_sec1_der(&private_key.secret_key).or_else(|_| anyhow::bail!("Invalid private key"))?;
        let shared_secret = p256::ecdh::diffie_hellman(secret_key.to_nonzero_scalar(), public_key.as_affine());
        Ok(shared_secret.raw_secret_bytes().to_vec())
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

    #[ignore]
    #[tokio::test]
    async fn simple_test() -> TestResult {
        let agreement1 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::EcDhP256).unwrap();
        let agreement2 = OmniAgreement::new(Utc::now(), OmniAgreementAlgorithmType::EcDhP256).unwrap();

        let public_key1 = agreement1.gen_agreement_public_key();
        let private_key1 = agreement1.gen_agreement_private_key();
        let public_key2 = agreement2.gen_agreement_public_key();
        let private_key2 = agreement2.gen_agreement_private_key();

        let secret1 = OmniAgreement::gen_secret(&private_key1, &public_key2)?;
        let secret2 = OmniAgreement::gen_secret(&private_key2, &public_key1)?;

        assert_eq!(secret1, secret2);

        println!("{:?}", secret1);

        Ok(())
    }
}
