use enumflags2::BitFlags;

use crate::prelude::*;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display, strum::FromRepr)]
pub(crate) enum AuthType {
    #[strum(serialize = "none")]
    None = 0,
    #[strum(serialize = "sign")]
    Sign = 1,
}

impl AuthType {
    pub const fn bits(self) -> u32 {
        self as u32
    }
}

#[repr(u32)]
#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display)]
pub(crate) enum KeyExchangeAlgorithmType {
    #[strum(serialize = "x25519")]
    X25519 = 1,
}

#[repr(u32)]
#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display)]
pub(crate) enum KeyDerivationAlgorithmType {
    #[strum(serialize = "hkdf")]
    Hkdf = 2,
}

#[repr(u32)]
#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display)]
pub(crate) enum CipherAlgorithmType {
    Aes256Gcm = 1,
}

#[repr(u32)]
#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::AsRefStr, strum::Display)]
pub(crate) enum HashAlgorithmType {
    Sha3_256 = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileMessage {
    pub session_id: Vec<u8>,
    pub auth_type: AuthType,
    pub key_exchange_algorithm_type_flags: BitFlags<KeyExchangeAlgorithmType>,
    pub key_derivation_algorithm_type_flags: BitFlags<KeyDerivationAlgorithmType>,
    pub cipher_algorithm_type_flags: BitFlags<CipherAlgorithmType>,
    pub hash_algorithm_type_flags: BitFlags<HashAlgorithmType>,
}

impl RocketPackStruct for ProfileMessage {
    fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
        encoder.write_map(6)?;

        encoder.write_u64(0)?;
        encoder.write_bytes(&value.session_id)?;

        encoder.write_u64(1)?;
        encoder.write_u32(value.auth_type as u32)?;

        encoder.write_u64(2)?;
        encoder.write_u32(value.key_exchange_algorithm_type_flags.bits())?;

        encoder.write_u64(3)?;
        encoder.write_u32(value.key_derivation_algorithm_type_flags.bits())?;

        encoder.write_u64(4)?;
        encoder.write_u32(value.cipher_algorithm_type_flags.bits())?;

        encoder.write_u64(5)?;
        encoder.write_u32(value.hash_algorithm_type_flags.bits())?;

        Ok(())
    }

    fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
    where
        Self: Sized,
    {
        let mut session_id: Vec<u8> = Vec::new();
        let mut auth_type = AuthType::None;
        let mut key_exchange_algorithm_type_flags = BitFlags::<KeyExchangeAlgorithmType>::empty();
        let mut key_derivation_algorithm_type_flags = BitFlags::<KeyDerivationAlgorithmType>::empty();
        let mut cipher_algorithm_type_flags = BitFlags::<CipherAlgorithmType>::empty();
        let mut hash_algorithm_type_flags = BitFlags::<HashAlgorithmType>::empty();

        let count = decoder.read_map()?;

        for _ in 0..count {
            match decoder.read_u64()? {
                0 => session_id = decoder.read_bytes_vec()?,
                1 => auth_type = AuthType::from_repr(decoder.read_u32()?).ok_or(RocketPackDecoderError::Other("parse error"))?,
                2 => key_exchange_algorithm_type_flags = BitFlags::<KeyExchangeAlgorithmType>::from_bits_truncate(decoder.read_u32()?),
                3 => key_derivation_algorithm_type_flags = BitFlags::<KeyDerivationAlgorithmType>::from_bits_truncate(decoder.read_u32()?),
                4 => cipher_algorithm_type_flags = BitFlags::<CipherAlgorithmType>::from_bits_truncate(decoder.read_u32()?),
                5 => hash_algorithm_type_flags = BitFlags::<HashAlgorithmType>::from_bits_truncate(decoder.read_u32()?),
                _ => decoder.skip_field()?,
            }
        }

        Ok(Self {
            session_id,
            auth_type,
            key_exchange_algorithm_type_flags,
            key_derivation_algorithm_type_flags,
            cipher_algorithm_type_flags,
            hash_algorithm_type_flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL};
    use enumflags2::make_bitflags;
    use testresult::TestResult;

    use super::*;

    #[ignore]
    #[test]
    fn simple_test() -> TestResult {
        let p = ProfileMessage {
            session_id: vec![1, 2, 3, 4],
            auth_type: AuthType::Sign,
            key_exchange_algorithm_type_flags: make_bitflags!(KeyExchangeAlgorithmType::X25519),
            key_derivation_algorithm_type_flags: make_bitflags!(KeyDerivationAlgorithmType::Hkdf),
            cipher_algorithm_type_flags: make_bitflags!(CipherAlgorithmType::Aes256Gcm),
            hash_algorithm_type_flags: make_bitflags!(HashAlgorithmType::Sha3_256),
        };

        let b = p.export()?;
        let p2 = ProfileMessage::import(&b.clone())?;

        assert_eq!(p, p2);

        let v = b.to_vec();
        println!("{:?}", BASE64_URL.encode(v.as_slice()));

        Ok(())
    }
}
