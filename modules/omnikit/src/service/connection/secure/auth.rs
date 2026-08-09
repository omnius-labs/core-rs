use std::sync::Arc;

use chrono::Utc;
use hkdf::SimpleHkdf;
use parking_lot::Mutex;
use rand::RngExt;
use sha3::{Digest, Sha3_256};

use omnius_core_base::clock::Clock;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};

use crate::{
    model::{OmniAgreement, OmniAgreementAlgorithmType, OmniAgreementPublicKey, OmniCert, OmniSigner},
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv, FramedSend, FramedSender},
};

use super::*;

#[allow(unused)]
pub(crate) struct Authenticator<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    typ: OmniSecureStreamType,
    receiver: FramedReceiver<ReadHalf<T>>,
    sender: FramedSender<WriteHalf<T>>,
    signer: Option<OmniSigner>,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    rng: Arc<Mutex<dyn rand::Rng + Send + Sync>>,
}

#[allow(unused)]
pub(crate) struct AuthResult {
    pub sign_id: Option<String>,
    pub cipher_algorithm_type: CipherAlgorithmType,
    pub enc_key: Vec<u8>,
    pub enc_nonce: Vec<u8>,
    pub dec_key: Vec<u8>,
    pub dec_nonce: Vec<u8>,
}

#[allow(unused)]
impl<T> Authenticator<T>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
{
    pub async fn new(
        typ: OmniSecureStreamType,
        reader: ReadHalf<T>,
        writer: WriteHalf<T>,
        max_frame_length: usize,
        signer: Option<OmniSigner>,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
        rng: Arc<Mutex<dyn rand::Rng + Send + Sync>>,
    ) -> Result<Self> {
        Ok(Self {
            typ,
            receiver: FramedReceiver::new(reader, max_frame_length),
            sender: FramedSender::new(writer, max_frame_length),
            signer,
            clock,
            rng,
        })
    }

    pub fn into_inner(self) -> (ReadHalf<T>, WriteHalf<T>) {
        (self.receiver.into_inner(), self.sender.into_inner())
    }

    pub async fn auth(&mut self) -> Result<AuthResult> {
        let my_profile = ProfileMessage {
            session_id: self.rng.lock().random::<[u8; 32]>().to_vec(),
            auth_type: match self.signer {
                Some(_) => AuthType::Sign,
                None => AuthType::None,
            },
            key_exchange_algorithm_type_flags: KEY_EXCHANGE_X25519,
            key_derivation_algorithm_type_flags: KEY_DERIVATION_HKDF,
            cipher_algorithm_type_flags: CIPHER_AES_256_GCM,
            hash_algorithm_type_flags: HASH_SHA3_256,
        };
        let other_profile = {
            self.sender.send(my_profile.export()?.into()).await?;
            ProfileMessage::import(&self.receiver.recv().await?)?
        };

        let key_exchange_algorithm_type_flags = my_profile.key_exchange_algorithm_type_flags & other_profile.key_exchange_algorithm_type_flags;
        let key_derivation_algorithm_type_flags = my_profile.key_derivation_algorithm_type_flags & other_profile.key_derivation_algorithm_type_flags;
        let cipher_algorithm_type_flags = my_profile.cipher_algorithm_type_flags & other_profile.cipher_algorithm_type_flags;
        let hash_algorithm_type_flags = my_profile.hash_algorithm_type_flags & other_profile.hash_algorithm_type_flags;

        let (other_sign, secret) = if has_flag(key_exchange_algorithm_type_flags, KEY_EXCHANGE_X25519) {
            let now = self.clock.now();
            let my_agreement = OmniAgreement::new(OmniAgreementAlgorithmType::X25519, now)?;
            let other_agreement_public_key = {
                self.sender.send(my_agreement.gen_agreement_public_key().export()?.into()).await?;
                OmniAgreementPublicKey::import(&self.receiver.recv().await?)?
            };

            if let Some(my_signer) = self.signer.as_ref() {
                let my_hash = Self::gen_hash(&my_profile, &my_agreement.gen_agreement_public_key(), &hash_algorithm_type_flags)?;
                let my_sign = my_signer.sign(&my_hash)?;
                self.sender.send(my_sign.export()?.into()).await?;
            }

            let other_sign = if other_profile.auth_type == AuthType::Sign {
                let other_cert = OmniCert::import(&self.receiver.recv().await?)?;
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

        let cipher_algorithm_type = if has_flag(cipher_algorithm_type_flags, CIPHER_AES_256_GCM) {
            CipherAlgorithmType::Aes256Gcm
        } else {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message("cipher algorithm"));
        };

        let (enc_key, enc_nonce, dec_key, dec_nonce) = if has_flag(key_derivation_algorithm_type_flags, KEY_DERIVATION_HKDF) {
            let salt = my_profile.session_id.iter().zip(other_profile.session_id.iter()).map(|(a, b)| a ^ b).collect::<Vec<u8>>();

            let (key_len, nonce_len) = match &cipher_algorithm_type {
                CipherAlgorithmType::Aes256Gcm => (32, 12),
            };

            let okm = if has_flag(hash_algorithm_type_flags, HASH_SHA3_256) {
                let mut okm = vec![0_u8; (key_len + nonce_len) * 2];
                let kdf = SimpleHkdf::<Sha3_256>::new(Some(&salt), &secret);
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
            sign_id: other_sign,
            cipher_algorithm_type,
            enc_key,
            enc_nonce,
            dec_key,
            dec_nonce,
        })
    }

    fn gen_hash(profile_message: &ProfileMessage, agreement_public_key: &OmniAgreementPublicKey, hash_algorithm: &u32) -> Result<Vec<u8>> {
        if has_flag(*hash_algorithm, HASH_SHA3_256) {
            let mut hasher = Sha3_256::new();
            hasher.update(profile_message.export()?);
            hasher.update(agreement_public_key.export()?);

            Ok(hasher.finalize().to_vec())
        } else {
            Err(Error::new(ErrorKind::UnsupportedType).with_message("hash algorithm"))
        }
    }
}

const fn has_flag(flags: u32, flag: u32) -> bool {
    flags & flag != 0
}
