use std::sync::Arc;

use chrono::Utc;
use enumflags2::{BitFlags, make_bitflags};
use hkdf::Hkdf;
use parking_lot::Mutex;
use sha3::{Digest, Sha3_256};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_base::{clock::Clock, random_bytes::RandomBytesProvider};

use crate::{
    model::{OmniAgreement, OmniAgreementAlgorithmType, OmniAgreementPublicKey, OmniCert, OmniSigner},
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::*;

#[allow(unused)]
pub(crate) struct Authenticator<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    typ: OmniSecureStreamType,
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    signer: Option<OmniSigner>,
    random_bytes_provider: Arc<Mutex<dyn RandomBytesProvider + Send + Sync>>,
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
        typ: OmniSecureStreamType,
        receiver: Arc<TokioMutex<FramedReceiver<R>>>,
        sender: Arc<TokioMutex<FramedSender<W>>>,
        signer: Option<OmniSigner>,
        random_bytes_provider: Arc<Mutex<dyn RandomBytesProvider + Send + Sync>>,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
    ) -> Result<Self> {
        Ok(Self {
            typ,
            receiver,
            sender,
            signer,
            random_bytes_provider,
            clock,
        })
    }

    pub async fn auth(&self) -> Result<AuthResult> {
        let my_profile = ProfileMessage {
            session_id: self.random_bytes_provider.lock().get_bytes(32),
            auth_type: match self.signer {
                Some(_) => AuthType::Sign,
                None => AuthType::None,
            },
            key_exchange_algorithm_type_flags: make_bitflags!(KeyExchangeAlgorithmType::X25519),
            key_derivation_algorithm_type_flags: make_bitflags!(KeyDerivationAlgorithmType::Hkdf),
            cipher_algorithm_type_flags: make_bitflags!(CipherAlgorithmType::Aes256Gcm),
            hash_algorithm_type_flags: make_bitflags!(HashAlgorithmType::Sha3_256),
        };
        let other_profile = {
            self.sender.lock().await.send(my_profile.export()?.into()).await?;
            ProfileMessage::import(&self.receiver.lock().await.recv().await?)?
        };

        let key_exchange_algorithm_type_flags = my_profile.key_exchange_algorithm_type_flags & other_profile.key_exchange_algorithm_type_flags;
        let key_derivation_algorithm_type_flags = my_profile.key_derivation_algorithm_type_flags & other_profile.key_derivation_algorithm_type_flags;
        let cipher_algorithm_type_flags = my_profile.cipher_algorithm_type_flags & other_profile.cipher_algorithm_type_flags;
        let hash_algorithm_type_flags = my_profile.hash_algorithm_type_flags & other_profile.hash_algorithm_type_flags;

        let (other_sign, secret) = if key_exchange_algorithm_type_flags.contains(KeyExchangeAlgorithmType::X25519) {
            let now = self.clock.now();
            let my_agreement = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, now)?;
            let other_agreement_public_key = {
                self.sender.lock().await.send(my_agreement.gen_agreement_public_key().export()?.into()).await?;
                OmniAgreementPublicKey::import(&self.receiver.lock().await.recv().await?)?
            };

            if let Some(my_signer) = self.signer.as_ref() {
                let my_hash = Self::gen_hash(&my_profile, &my_agreement.gen_agreement_public_key(), &hash_algorithm_type_flags)?;
                let my_sign = my_signer.sign(&my_hash)?;
                self.sender.lock().await.send(my_sign.export()?.into()).await?;
            }

            let other_sign = if other_profile.auth_type == AuthType::Sign {
                let other_cert = OmniCert::import(&self.receiver.lock().await.recv().await?)?;
                let other_hash = Self::gen_hash(&other_profile, &other_agreement_public_key, &hash_algorithm_type_flags)?;
                other_cert.verify(&other_hash)?;

                Some(other_cert.to_string())
            } else {
                None
            };

            let secret = OmniAgreement::gen_secret(&my_agreement.gen_agreement_private_key(), &other_agreement_public_key)?;

            (other_sign, secret)
        } else {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("key exchange algorithm"));
        };

        let cipher_algorithm_type = if cipher_algorithm_type_flags.contains(CipherAlgorithmType::Aes256Gcm) {
            CipherAlgorithmType::Aes256Gcm
        } else {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("cipher algorithm"));
        };

        let (enc_key, enc_nonce, dec_key, dec_nonce) = if key_derivation_algorithm_type_flags.contains(KeyDerivationAlgorithmType::Hkdf) {
            let salt = my_profile.session_id.iter().zip(other_profile.session_id.iter()).map(|(a, b)| a ^ b).collect::<Vec<u8>>();

            let (key_len, nonce_len) = match cipher_algorithm_type {
                CipherAlgorithmType::Aes256Gcm => (32, 12),
            };

            let okm = if hash_algorithm_type_flags.contains(HashAlgorithmType::Sha3_256) {
                let mut okm = vec![0_u8; (key_len + nonce_len) * 2];
                let kdf = Hkdf::<Sha3_256>::new(Some(&salt), &secret);
                kdf.expand(&[], &mut okm)
                    .map_err(|_| Error::new(ErrorKind::InvalidFormat).with_message("Failed to expand key"))?;

                okm
            } else {
                return Err(Error::new(ErrorKind::UnsupportedType).with_message("hash algorithm"));
            };

            let (enc_offset, dec_offset) = match self.typ {
                OmniSecureStreamType::Connected => (0, key_len + nonce_len),
                OmniSecureStreamType::Accepted => (key_len + nonce_len, 0),
            };

            let enc_key = okm[enc_offset..(enc_offset + key_len)].to_vec();
            let enc_nonce = okm[(enc_offset + key_len)..(enc_offset + key_len + nonce_len)].to_vec();
            let dec_key = okm[dec_offset..(dec_offset + key_len)].to_vec();
            let dec_nonce = okm[(dec_offset + key_len)..(dec_offset + key_len + nonce_len)].to_vec();

            (enc_key, enc_nonce, dec_key, dec_nonce)
        } else {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("key derivation algorithm"));
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

    fn gen_hash(profile_message: &ProfileMessage, agreement_public_key: &OmniAgreementPublicKey, hash_algorithm: &BitFlags<HashAlgorithmType>) -> Result<Vec<u8>> {
        if hash_algorithm.contains(HashAlgorithmType::Sha3_256) {
            let mut hasher = Sha3_256::new();
            hasher.update(&profile_message.session_id);
            hasher.update(profile_message.auth_type.bits().to_le_bytes());
            hasher.update(profile_message.key_exchange_algorithm_type_flags.bits().to_le_bytes());
            hasher.update(profile_message.key_derivation_algorithm_type_flags.bits().to_le_bytes());
            hasher.update(profile_message.cipher_algorithm_type_flags.bits().to_le_bytes());
            hasher.update(profile_message.hash_algorithm_type_flags.bits().to_le_bytes());
            hasher.update(agreement_public_key.created_time.timestamp().to_be_bytes());
            hasher.update(agreement_public_key.algorithm_type.bits().to_le_bytes());
            hasher.update(&agreement_public_key.public_key);

            Ok(hasher.finalize().to_vec())
        } else {
            Err(Error::new(ErrorKind::UnsupportedType).with_message("hash algorithm"))
        }
    }
}
