use enumflags2::{BitFlag, BitFlags, bitflags};

use crate::generated::{
    omni_agreement::OmniAgreementAlgorithmType,
    omni_secure::{AuthType, ProfileMessage},
};

impl ProfileMessage {
    pub fn new(
        session_id: Vec<u8>,
        auth_type: AuthType,
        key_exchange_algorithm_type: BitFlags<KeyExchangeAlgorithmType>,
        key_derivation_algorithm_type: BitFlags<KeyDerivationAlgorithmType>,
        cipher_algorithm_type: BitFlags<CipherAlgorithmType>,
        hash_algorithm_type: BitFlags<HashAlgorithmType>,
    ) -> Self {
        let mut v = ProfileMessage {
            session_id,
            auth_type,
            key_exchange_algorithm_type_flags: 0,
            key_derivation_algorithm_type_flags: 0,
            cipher_algorithm_type_flags: 0,
            hash_algorithm_type_flags: 0,
        };
        v.set_key_exchange_algorithm_type(key_exchange_algorithm_type);
        v.set_key_derivation_algorithm_type(key_derivation_algorithm_type);
        v.set_cipher_algorithm_type(cipher_algorithm_type);
        v.set_hash_algorithm_type(hash_algorithm_type);
        v
    }

    pub fn get_key_exchange_algorithm_type(&self) -> BitFlags<KeyExchangeAlgorithmType> {
        KeyExchangeAlgorithmType::from_bits_truncate(self.key_exchange_algorithm_type_flags)
    }

    pub fn set_key_exchange_algorithm_type(&mut self, flags: BitFlags<KeyExchangeAlgorithmType>) {
        self.key_exchange_algorithm_type_flags = flags.bits();
    }

    pub fn get_key_derivation_algorithm_type(&self) -> BitFlags<KeyDerivationAlgorithmType> {
        KeyDerivationAlgorithmType::from_bits_truncate(self.key_derivation_algorithm_type_flags)
    }

    pub fn set_key_derivation_algorithm_type(&mut self, flags: BitFlags<KeyDerivationAlgorithmType>) {
        self.key_derivation_algorithm_type_flags = flags.bits();
    }

    pub fn get_cipher_algorithm_type(&self) -> BitFlags<CipherAlgorithmType> {
        CipherAlgorithmType::from_bits_truncate(self.cipher_algorithm_type_flags)
    }

    pub fn set_cipher_algorithm_type(&mut self, flags: BitFlags<CipherAlgorithmType>) {
        self.cipher_algorithm_type_flags = flags.bits();
    }

    pub fn get_hash_algorithm_type(&self) -> BitFlags<HashAlgorithmType> {
        HashAlgorithmType::from_bits_truncate(self.hash_algorithm_type_flags)
    }

    pub fn set_hash_algorithm_type(&mut self, flags: BitFlags<HashAlgorithmType>) {
        self.hash_algorithm_type_flags = flags.bits();
    }
}

impl AuthType {
    pub fn to_u32(&self) -> u32 {
        match self {
            AuthType::None => 1,
            AuthType::Sign => 2,
        }
    }
}

impl OmniAgreementAlgorithmType {
    pub fn to_u32(&self) -> u32 {
        match self {
            OmniAgreementAlgorithmType::None => 1,
            OmniAgreementAlgorithmType::X25519 => 2,
        }
    }
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyExchangeAlgorithmType {
    X25519 = 1,
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyDerivationAlgorithmType {
    HKDF = 1,
}

#[allow(nonstandard_style)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CipherAlgorithmType {
    AES_256_GCM = 1,
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HashAlgorithmType {
    SHA3_256 = 1,
}
