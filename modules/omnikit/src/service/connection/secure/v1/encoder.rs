use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit as _};
use sha3::digest::generic_array::GenericArray;

use super::util::increment_bytes;

#[allow(unused)]
pub(crate) struct Aes256GcmEncoder {
    cipher: Aes256Gcm,
    nonce: Vec<u8>,
}

#[allow(unused)]
impl Aes256GcmEncoder {
    pub fn new(key: &[u8], nonce: &[u8]) -> Self {
        Self {
            cipher: Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)),
            nonce: nonce.to_vec(),
        }
    }

    pub fn encode(&mut self, data: &[u8]) -> Vec<u8> {
        let nonce = GenericArray::from_slice(self.nonce.as_slice());
        let encrypted_payload = self.cipher.encrypt(nonce, data).expect("Failed to encrypt");

        increment_bytes(&mut self.nonce);

        encrypted_payload
    }
}
