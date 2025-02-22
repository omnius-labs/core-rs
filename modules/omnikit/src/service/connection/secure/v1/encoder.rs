use aes_gcm::{Aes256Gcm, Error, Key, KeyInit as _, aead::Aead};
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

    pub fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = GenericArray::from_slice(self.nonce.as_slice());
        let ciphertext_with_tag = self.cipher.encrypt(nonce, data)?;

        increment_bytes(&mut self.nonce);

        Ok(ciphertext_with_tag)
    }
}
