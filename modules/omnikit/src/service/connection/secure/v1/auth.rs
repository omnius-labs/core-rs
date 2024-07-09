use std::sync::Arc;

use chrono::Utc;
use hkdf::Hkdf;
use rand::{RngCore as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use sha3::{Digest, Sha3_256};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_base::clock::Clock;

use crate::{
    connection::{
        framed::{FramedReceiver, FramedSender},
        secure::OmniSecureStreamVersion,
    },
    OmniAgreement, OmniAgreementAlgorithmType, OmniAgreementPublicKey, OmniCert, OmniSigner,
};

use super::*;

#[allow(unused)]
pub(crate) struct Authenticator<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    version: OmniSecureStreamVersion,
    typ: OmniSecureStreamType,
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    signer: Option<OmniSigner>,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
}

#[allow(unused)]
pub(crate) struct AuthResult {
    pub sign: Option<String>,
    pub cipher_algorithm_type: CipherAlgorithmType,
    pub enc_key: Vec<u8>,
    pub enc_nonce: Vec<u8>,
    pub dec_key: Vec<u8>,
    pub dec_nonce: Vec<u8>,
}

#[allow(unused)]
impl<R, W> Authenticator<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    pub async fn new(
        version: OmniSecureStreamVersion,
        typ: OmniSecureStreamType,
        receiver: Arc<TokioMutex<FramedReceiver<R>>>,
        sender: Arc<TokioMutex<FramedSender<W>>>,
        signer: Option<OmniSigner>,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            version,
            typ,
            receiver,
            sender,
            signer,
            clock,
        })
    }

    pub async fn auth(&self) -> anyhow::Result<AuthResult> {
        let my_profile = ProfileMessage {
            version: self.version.clone(),
            session_id: Self::gen_id(),
            auth_type: match self.signer {
                Some(_) => AuthType::Sign,
                None => AuthType::None,
            },
            key_exchange_algorithm_type: KeyExchangeAlgorithmType::EcDhP521,
            key_derivation_algorithm_type: KeyDerivationAlgorithmType::Hkdf,
            cipher_algorithm_type: CipherAlgorithmType::Aes256Gcm,
            hash_algorithm_type: HashAlgorithmType::Sha3_256,
        };
        let other_profile = {
            self.sender.lock().await.send_message(&my_profile).await?;
            let profile: ProfileMessage = self.receiver.lock().await.recv_message().await?;

            profile
        };

        let key_exchange_algorithm_type = my_profile.key_exchange_algorithm_type.clone() | other_profile.key_exchange_algorithm_type.clone();
        let key_derivation_algorithm_type = my_profile.key_derivation_algorithm_type.clone() | other_profile.key_derivation_algorithm_type.clone();
        let cipher_algorithm_type = my_profile.cipher_algorithm_type.clone() | other_profile.cipher_algorithm_type.clone();
        let hash_algorithm_type = my_profile.hash_algorithm_type.clone() | other_profile.hash_algorithm_type.clone();

        let (other_sign, secret) = match key_exchange_algorithm_type {
            KeyExchangeAlgorithmType::EcDhP521 => {
                let now = self.clock.now();
                let my_agreement = OmniAgreement::new(now, OmniAgreementAlgorithmType::EcDhP256)?;
                let other_agreement_public_key = {
                    self.sender.lock().await.send_message(my_agreement.gen_agreement_public_key()).await?;
                    let agreement_public_key: OmniAgreementPublicKey = self.receiver.lock().await.recv_message().await?;

                    agreement_public_key
                };

                if let Some(my_signer) = self.signer.as_ref() {
                    let my_hash = Self::gen_hash(&my_profile, &my_agreement.gen_agreement_public_key(), &hash_algorithm_type)?;
                    let my_sign = my_signer.sign(&my_hash)?;
                    self.sender.lock().await.send_message(my_sign).await?;
                }

                let other_sign = if other_profile.auth_type == AuthType::Sign {
                    let other_cert: OmniCert = self.receiver.lock().await.recv_message().await?;
                    let other_hash = Self::gen_hash(&other_profile, &other_agreement_public_key, &hash_algorithm_type)?;
                    other_cert.verify(&other_hash)?;

                    Some(other_cert.to_string())
                } else {
                    None
                };

                let secret = OmniAgreement::gen_secret(&my_agreement.gen_agreement_private_key(), &other_agreement_public_key)?;

                (other_sign, secret)
            }
            _ => anyhow::bail!("Invalid key exchange algorithm"),
        };

        let (enc_key, enc_nonce, dec_key, dec_nonce) = match key_derivation_algorithm_type {
            KeyDerivationAlgorithmType::Hkdf => {
                let salt = my_profile
                    .session_id
                    .iter()
                    .zip(other_profile.session_id.iter())
                    .map(|(a, b)| a ^ b)
                    .collect::<Vec<u8>>();

                let (key_len, nonce_len) = match cipher_algorithm_type {
                    CipherAlgorithmType::Aes256Gcm => (32, 12),
                    _ => anyhow::bail!("Invalid cipher algorithm"),
                };

                let okm = match hash_algorithm_type {
                    HashAlgorithmType::Sha3_256 => {
                        let mut okm = vec![0_u8; (key_len + nonce_len) * 2];
                        let kdf = Hkdf::<Sha3_256>::new(Some(&salt), &secret);
                        kdf.expand(&[], &mut okm).or_else(|_| anyhow::bail!("Failed to expand key"))?;

                        okm
                    }
                    _ => anyhow::bail!("Invalid hash algorithm"),
                };

                let (enc_offset, dec_offset) = match self.typ {
                    OmniSecureStreamType::Connected => (0, key_len + nonce_len),
                    OmniSecureStreamType::Accepted => (key_len + nonce_len, 0),
                };

                let enc_key = okm[enc_offset..enc_offset + key_len].to_vec();
                let enc_nonce = okm[enc_offset + key_len..enc_offset + key_len + nonce_len].to_vec();
                let dec_key = okm[dec_offset..dec_offset + key_len].to_vec();
                let dec_nonce = okm[dec_offset + key_len..dec_offset + key_len + nonce_len].to_vec();

                (enc_key, enc_nonce, dec_key, dec_nonce)
            }
            _ => anyhow::bail!("Invalid key derivation algorithm"),
        };

        Ok(AuthResult {
            sign: other_sign,
            cipher_algorithm_type,
            enc_key,
            enc_nonce,
            dec_key,
            dec_nonce,
        })
    }

    fn gen_id() -> Vec<u8> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut id = [0_u8, 32];
        rng.fill_bytes(&mut id);
        id.to_vec()
    }

    fn gen_hash(
        profile_message: &ProfileMessage,
        agreement_public_key: &OmniAgreementPublicKey,
        hash_algorithm: &HashAlgorithmType,
    ) -> anyhow::Result<Vec<u8>> {
        match hash_algorithm {
            &HashAlgorithmType::Sha3_256 => {
                let mut hasher = Sha3_256::new();
                hasher.update(&profile_message.session_id);
                hasher.update(profile_message.auth_type.bits().to_le_bytes());
                hasher.update(profile_message.key_exchange_algorithm_type.bits().to_le_bytes());
                hasher.update(profile_message.key_derivation_algorithm_type.bits().to_le_bytes());
                hasher.update(profile_message.cipher_algorithm_type.bits().to_le_bytes());
                hasher.update(profile_message.hash_algorithm_type.bits().to_le_bytes());
                hasher.update(agreement_public_key.created_time.timestamp().to_be_bytes());
                hasher.update(agreement_public_key.algorithm_type.bits().to_le_bytes());
                hasher.update(&agreement_public_key.public_key);

                Ok(hasher.finalize().to_vec())
            }
            _ => anyhow::bail!("Invalid hash algorithm"),
        }
    }
}
