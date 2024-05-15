use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::connection::omni_secure::OmniSecureStreamVersion;

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct AuthType: u32 {
        const None = 0;
        const Sign = 1;
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyExchangeAlgorithmType: u32 {
        const None = 0;
        const EcDhP521 = 1;
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct KeyDerivationAlgorithmType: u32 {
        const None = 0;
        const Hkdf = 1;
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct CipherAlgorithmType: u32 {
        const None = 0;
        const Aes256Gcm = 1;
    }
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) struct HashAlgorithmType: u32 {
        const None = 0;
        const Sha3_256 = 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProfileMessage {
    pub version: OmniSecureStreamVersion,
    pub session_id: Vec<u8>,
    pub auth_type: AuthType,
    pub key_exchange_algorithm_type: KeyExchangeAlgorithmType,
    pub key_derivation_algorithm_type: KeyDerivationAlgorithmType,
    pub cipher_algorithm_type: CipherAlgorithmType,
    pub hash_algorithm_type: HashAlgorithmType,
}
