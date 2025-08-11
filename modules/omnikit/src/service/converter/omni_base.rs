use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL};

use crate::prelude::*;

// ref. https://github.com/multiformats/multibase#multibase-table

pub struct OmniBase;

impl OmniBase {
    pub fn encode_by_base16(data: &[u8]) -> String {
        let mut bytes = vec![0; 1 + (data.len() * 2)];
        bytes[0] = b'f';
        hex::encode_to_slice(data, &mut bytes[1..]).unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    }

    pub fn encode_by_base64_url(data: &[u8]) -> String {
        let mut bytes = vec![0; 1 + (data.len() * 4 / 3 + 4)];
        bytes[0] = b'u';
        let len = BASE64_URL.encode_slice(data, &mut bytes[1..]).unwrap();
        String::from_utf8_lossy(&bytes[..(1 + len)]).to_string()
    }

    pub fn decode(data: &str) -> Result<Vec<u8>> {
        let data = data.as_bytes();
        if data.len() <= 1 {
            return Err(Error::builder().kind(ErrorKind::InvalidFormat).message("omni base too small").build());
        }

        match data[0] {
            b'f' => hex::decode(&data[1..]).map_err(|e| e.into()),
            b'u' => BASE64_URL.decode(&data[1..]).map_err(|e| e.into()),
            _ => Err(Error::builder().kind(ErrorKind::InvalidFormat).message("invalid omni base").build()),
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn base16_test() -> TestResult {
        let b16 = OmniBase::encode_by_base16(b"test");
        println!("{b16}");
        assert_eq!(OmniBase::decode(b16.as_str())?, b"test".to_vec());
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn base64_url_test() -> TestResult {
        let b64 = OmniBase::encode_by_base64_url(b"test");
        println!("{b64}");
        assert_eq!(OmniBase::decode(b64.as_str())?, b"test".to_vec());
        Ok(())
    }
}
