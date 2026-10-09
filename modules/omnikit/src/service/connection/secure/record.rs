use std::io;

use aes_gcm::{
    Aes256Gcm, KeyInit as _,
    aead::{Aead, Payload},
};
use zeroize::Zeroizing;

use super::kdf::V2Kdf;
use crate::generated::omni_secure::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RecordKind {
    Data = 1,
    KeyUpdate = 2,
    Close = 3,
}

#[derive(Clone, Copy)]
pub(super) struct RecordHeader {
    pub kind: RecordKind,
    pub generation: u64,
    pub sequence: u64,
    pub plaintext_length: usize,
}

impl RecordHeader {
    pub const LENGTH: usize = V2_RECORD_HEADER_LENGTH as usize;

    pub fn parse(bytes: &[u8; Self::LENGTH]) -> io::Result<Self> {
        let kind = match bytes[0] {
            1 => RecordKind::Data,
            2 => RecordKind::KeyUpdate,
            3 => RecordKind::Close,
            _ => return Err(Self::invalid("unknown record kind")),
        };
        let generation = u64::from_le_bytes(bytes[1..9].try_into().map_err(|_| Self::invalid("generation header"))?);
        let sequence = u64::from_le_bytes(bytes[9..17].try_into().map_err(|_| Self::invalid("sequence header"))?);
        let ciphertext_length = u32::from_le_bytes(bytes[17..21].try_into().map_err(|_| Self::invalid("length header"))?) as usize;
        let plaintext_length = ciphertext_length.checked_sub(16).ok_or_else(|| Self::invalid("short authentication tag"))?;
        let value = Self {
            kind,
            generation,
            sequence,
            plaintext_length,
        };
        value.validate_length()?;
        Ok(value)
    }

    pub fn bytes(&self) -> [u8; Self::LENGTH] {
        let mut bytes = [0; Self::LENGTH];
        bytes[0] = self.kind as u8;
        bytes[1..9].copy_from_slice(&self.generation.to_le_bytes());
        bytes[9..17].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[17..21].copy_from_slice(&((self.plaintext_length + 16) as u32).to_le_bytes());
        bytes
    }

    fn validate_length(&self) -> io::Result<()> {
        let valid = match self.kind {
            RecordKind::Data => (1..=V2_RECORD_MAX_PLAINTEXT_LENGTH as usize).contains(&self.plaintext_length),
            RecordKind::KeyUpdate => self.plaintext_length == 8,
            RecordKind::Close => self.plaintext_length == 0,
        };
        if !valid {
            return Err(Self::invalid("record length does not match kind"));
        }
        Ok(())
    }

    pub(super) fn invalid(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
}

/// 一方向・一世代の秘密と使用量。Debug を実装せず秘密を出力しない。
pub(super) struct TrafficState {
    secret: Zeroizing<[u8; 32]>,
    key: Zeroizing<[u8; 32]>,
    iv: Zeroizing<[u8; 12]>,
    transcript: [u8; 32],
    direction: u8,
    generation: u64,
    sequence: u64,
    bytes_used: u64,
}

impl TrafficState {
    pub fn new(secret: Zeroizing<[u8; 32]>, transcript: [u8; 32], direction: u8) -> Self {
        Self::with_generation(secret, transcript, direction, 0)
    }

    fn with_generation(secret: Zeroizing<[u8; 32]>, transcript: [u8; 32], direction: u8, generation: u64) -> Self {
        let generation_bytes = generation.to_le_bytes();
        let direction_bytes = [direction];
        let suffix: &[&[u8]] = &[&transcript, &direction_bytes, &generation_bytes];
        let mut key_info: Vec<&[u8]> = vec![b"omnius.secure.v2/key\0"];
        key_info.extend_from_slice(suffix);
        let mut iv_info: Vec<&[u8]> = vec![b"omnius.secure.v2/iv\0"];
        iv_info.extend_from_slice(suffix);
        let key = V2Kdf::expand::<32>(secret.as_slice(), &key_info);
        let iv = V2Kdf::expand::<12>(secret.as_slice(), &iv_info);
        Self {
            secret,
            key,
            iv,
            transcript,
            direction,
            generation,
            sequence: 0,
            bytes_used: 0,
        }
    }

    pub fn validate_header(&self, header: &RecordHeader) -> io::Result<()> {
        header.validate_length()?;
        if header.generation != self.generation || header.sequence != self.sequence {
            return Err(RecordHeader::invalid("unexpected generation or sequence"));
        }
        let next_bytes = self
            .bytes_used
            .checked_add(header.plaintext_length as u64)
            .ok_or_else(|| RecordHeader::invalid("record usage overflow"))?;
        if next_bytes > V2_EPOCH_MAX_PLAINTEXT_LENGTH || self.sequence >= V2_EPOCH_MAX_RECORD_COUNT {
            return Err(RecordHeader::invalid("epoch usage exceeded"));
        }
        if header.kind == RecordKind::Data && (next_bytes > V2_EPOCH_MAX_PLAINTEXT_LENGTH - 8 || self.sequence >= V2_EPOCH_MAX_RECORD_COUNT - 1) {
            return Err(RecordHeader::invalid("update reservation exceeded"));
        }
        if header.kind == RecordKind::KeyUpdate && self.sequence == 0 {
            return Err(RecordHeader::invalid("key update without data"));
        }
        Ok(())
    }

    pub fn data_capacity(&self, byte_limit: u64, record_limit: u64) -> usize {
        if self.sequence >= record_limit - 1 {
            return 0;
        }
        (byte_limit - 8).saturating_sub(self.bytes_used).min(V2_RECORD_MAX_PLAINTEXT_LENGTH as u64) as usize
    }

    #[cfg(test)]
    pub(super) fn set_test_state(&mut self, generation: u64, sequence: u64, bytes_used: u64) {
        *self = Self::with_generation(Zeroizing::new(*self.secret), self.transcript, self.direction, generation);
        self.sequence = sequence;
        self.bytes_used = bytes_used;
    }

    pub fn next_generation(&self) -> io::Result<u64> {
        self.generation.checked_add(1).ok_or_else(|| RecordHeader::invalid("generation exhausted"))
    }

    pub fn advance(&mut self) -> io::Result<()> {
        let next = self.next_generation()?;
        let secret = V2Kdf::expand::<32>(
            self.secret.as_slice(),
            &[b"omnius.secure.v2/update\0", &self.transcript, &[self.direction], &next.to_le_bytes()],
        );
        *self = Self::with_generation(secret, self.transcript, self.direction, next);
        Ok(())
    }

    fn nonce(&self) -> Zeroizing<[u8; 12]> {
        let mut nonce = Zeroizing::new(*self.iv);
        for (byte, count) in nonce[4..].iter_mut().zip(self.sequence.to_be_bytes()) {
            *byte ^= count;
        }
        nonce
    }

    fn aad(&self, header: &RecordHeader) -> Vec<u8> {
        let mut aad = b"omnius.secure.v2/record\0".to_vec();
        aad.extend_from_slice(&self.transcript);
        aad.push(self.direction);
        aad.extend_from_slice(&header.bytes());
        aad
    }

    #[allow(deprecated)]
    pub fn encode(&mut self, kind: RecordKind, plaintext: &[u8]) -> io::Result<Vec<u8>> {
        let header = RecordHeader {
            kind,
            generation: self.generation,
            sequence: self.sequence,
            plaintext_length: plaintext.len(),
        };
        self.validate_header(&header)?;
        if kind == RecordKind::KeyUpdate && plaintext != self.next_generation()?.to_le_bytes() {
            return Err(RecordHeader::invalid("wrong update generation"));
        }
        let cipher = Aes256Gcm::new_from_slice(self.key.as_slice()).map_err(|_| RecordHeader::invalid("invalid AES key"))?;
        let nonce = self.nonce();
        let aad = self.aad(&header);
        let ciphertext = cipher
            .encrypt(aes_gcm::Nonce::from_slice(nonce.as_slice()), Payload { msg: plaintext, aad: &aad })
            .map_err(|_| RecordHeader::invalid("record encryption failed"))?;
        let mut wire = header.bytes().to_vec();
        wire.extend_from_slice(&ciphertext);
        self.sequence += 1;
        self.bytes_used += plaintext.len() as u64;
        Ok(wire)
    }

    #[allow(deprecated)]
    pub fn decode(&mut self, header: &RecordHeader, ciphertext: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
        self.validate_header(header)?;
        if ciphertext.len() != header.plaintext_length + 16 {
            return Err(RecordHeader::invalid("ciphertext length mismatch"));
        }
        let cipher = Aes256Gcm::new_from_slice(self.key.as_slice()).map_err(|_| RecordHeader::invalid("invalid AES key"))?;
        let nonce = self.nonce();
        let aad = self.aad(header);
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(aes_gcm::Nonce::from_slice(nonce.as_slice()), Payload { msg: ciphertext, aad: &aad })
                .map_err(|_| RecordHeader::invalid("record authentication failed"))?,
        );
        if header.kind == RecordKind::KeyUpdate {
            if plaintext.as_slice() != self.next_generation()?.to_le_bytes() {
                return Err(RecordHeader::invalid("wrong update generation"));
            }
            self.advance()?;
        } else {
            self.sequence += 1;
            self.bytes_used += plaintext.len() as u64;
        }
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[test]
    fn final_data_reserves_update_bytes_and_record_before_rekey() -> TestResult {
        for byte_boundary in [false, true] {
            let mut sender = TrafficState::new(Zeroizing::new([1; 32]), [2; 32], 1);
            let mut receiver = TrafficState::new(Zeroizing::new([1; 32]), [2; 32], 1);
            if byte_boundary {
                sender.bytes_used = V2_EPOCH_MAX_PLAINTEXT_LENGTH - 9;
                receiver.bytes_used = sender.bytes_used;
            } else {
                sender.sequence = V2_EPOCH_MAX_RECORD_COUNT - 2;
                receiver.sequence = sender.sequence;
            }
            let data = sender.encode(RecordKind::Data, &[7])?;
            let header = RecordHeader::parse(data[..21].try_into().unwrap())?;
            assert_eq!(receiver.decode(&header, &data[21..])?.as_slice(), &[7]);
            assert_eq!(sender.data_capacity(V2_EPOCH_MAX_PLAINTEXT_LENGTH, V2_EPOCH_MAX_RECORD_COUNT), 0);
            assert!(sender.encode(RecordKind::Data, &[8]).is_err());
            let update = sender.encode(RecordKind::KeyUpdate, &1u64.to_le_bytes())?;
            let header = RecordHeader::parse(update[..21].try_into().unwrap())?;
            receiver.decode(&header, &update[21..])?;
            sender.advance()?;
            let data = sender.encode(RecordKind::Data, &[8])?;
            let header = RecordHeader::parse(data[..21].try_into().unwrap())?;
            assert_eq!(header.generation, 1);
            assert_eq!(header.sequence, 0);
            assert_eq!(receiver.decode(&header, &data[21..])?.as_slice(), &[8]);
        }
        Ok(())
    }

    #[test]
    fn generation_exhaustion_and_reserved_slot_reject_invalid_updates() {
        let mut state = TrafficState::new(Zeroizing::new([1; 32]), [2; 32], 1);
        assert!(state.encode(RecordKind::KeyUpdate, &1u64.to_le_bytes()).is_err());
        state.generation = u64::MAX;
        state.sequence = 1;
        assert!(state.next_generation().is_err());
        assert!(state.advance().is_err());
        state.generation = 0;
        state.sequence = V2_EPOCH_MAX_RECORD_COUNT - 1;
        assert!(state.encode(RecordKind::Data, &[1]).is_err());
        assert!(state.encode(RecordKind::Close, &[]).is_ok());
        assert!(state.encode(RecordKind::Close, &[]).is_err());
    }
}
