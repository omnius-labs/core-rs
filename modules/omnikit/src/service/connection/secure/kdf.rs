use sha3::{Digest, Sha3_256};
use zeroize::Zeroizing;

/// SHA3-256 専用の導出。HMAC の pad と中間出力を消去可能な所有者に置く。
pub(super) struct V2Kdf;

impl V2Kdf {
    pub fn hash(parts: &[&[u8]]) -> [u8; 32] {
        let mut hash = Sha3_256::new();
        for part in parts {
            hash.update(part);
        }
        hash.finalize().into()
    }

    pub fn hmac(key: &[u8], parts: &[&[u8]]) -> Zeroizing<[u8; 32]> {
        let mut inner_pad = Zeroizing::new([0u8; 136]);
        if key.len() > inner_pad.len() {
            let reduced = Zeroizing::new(Self::hash(&[key]));
            inner_pad[..32].copy_from_slice(reduced.as_slice());
        } else {
            inner_pad[..key.len()].copy_from_slice(key);
        }
        let mut outer_pad = Zeroizing::new(*inner_pad);
        for byte in inner_pad.iter_mut() {
            *byte ^= 0x36;
        }
        for byte in outer_pad.iter_mut() {
            *byte ^= 0x5c;
        }
        let mut inner = Sha3_256::new();
        inner.update(inner_pad.as_slice());
        for part in parts {
            inner.update(part);
        }
        let inner_output = Zeroizing::new(<[u8; 32]>::from(inner.finalize()));
        Zeroizing::new(Self::hash(&[outer_pad.as_slice(), inner_output.as_slice()]))
    }

    pub fn expand<const N: usize>(secret: &[u8], info: &[&[u8]]) -> Zeroizing<[u8; N]> {
        const {
            assert!(N <= 32);
        }
        let mut parts = info.to_vec();
        parts.push(&[1]);
        let block = Self::hmac(secret, &parts);
        let mut result = Zeroizing::new([0u8; N]);
        result.copy_from_slice(&block[..N]);
        result
    }
}
