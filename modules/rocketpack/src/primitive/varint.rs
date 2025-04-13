use std::mem::transmute;

use tokio_util::bytes::{Buf, BufMut};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum VarintError {
    #[error("Invalid header (value {value})")]
    InvalidHeader { value: u8 },

    #[error("End of input")]
    EndOfInput,

    #[error("Too small body (size: {size})")]
    TooSmall { size: usize },
}

type Result<T> = std::result::Result<T, VarintError>;

pub struct Varint;

impl Varint {
    pub const MIN_INT7: u8 = 0x00; // 0
    pub const MAX_INT7: u8 = 0x7F; // 127

    const INT8_CODE: u8 = 0x80;
    const INT16_CODE: u8 = 0x81;
    const INT32_CODE: u8 = 0x82;
    const INT64_CODE: u8 = 0x83;

    pub fn put_u8(value: u8, writer: &mut impl BufMut) {
        if value <= Self::MAX_INT7 {
            writer.put_u8(value);
        } else {
            writer.put_u8(Self::INT8_CODE);
            writer.put_u8(value);
        }
    }

    pub fn put_u16(value: u16, writer: &mut impl BufMut) {
        if value <= Self::MAX_INT7 as u16 {
            writer.put_u8(value as u8);
        } else if value <= u8::MAX as u16 {
            writer.put_u8(Self::INT8_CODE);
            writer.put_u8(value as u8);
        } else {
            writer.put_u8(Self::INT16_CODE);
            writer.put_u16_le(value);
        }
    }

    pub fn put_u32(value: u32, writer: &mut impl BufMut) {
        if value <= Self::MAX_INT7 as u32 {
            writer.put_u8(value as u8);
        } else if value <= u8::MAX as u32 {
            writer.put_u8(Self::INT8_CODE);
            writer.put_u8(value as u8);
        } else if value <= u16::MAX as u32 {
            writer.put_u8(Self::INT16_CODE);
            writer.put_u16_le(value as u16);
        } else {
            writer.put_u8(Self::INT32_CODE);
            writer.put_u32_le(value);
        }
    }

    pub fn put_u64(value: u64, writer: &mut impl BufMut) {
        if value <= Self::MAX_INT7 as u64 {
            writer.put_u8(value as u8);
        } else if value <= u8::MAX as u64 {
            writer.put_u8(Self::INT8_CODE);
            writer.put_u8(value as u8);
        } else if value <= u16::MAX as u64 {
            writer.put_u8(Self::INT16_CODE);
            writer.put_u16_le(value as u16);
        } else if value <= u32::MAX as u64 {
            writer.put_u8(Self::INT32_CODE);
            writer.put_u32_le(value as u32);
        } else {
            writer.put_u8(Self::INT64_CODE);
            writer.put_u64_le(value);
        }
    }

    pub fn put_i8(value: i8, writer: &mut impl BufMut) {
        let value: u8 = unsafe { transmute(value) };
        let value = (value << 1) ^ (value >> 7);
        Self::put_u8(value, writer);
    }

    pub fn put_i16(value: i16, writer: &mut impl BufMut) {
        let value: u16 = unsafe { transmute(value) };
        let value = (value << 1) ^ (value >> 15);
        Self::put_u16(value, writer);
    }

    pub fn put_i32(value: i32, writer: &mut impl BufMut) {
        let value: u32 = unsafe { transmute(value) };
        let value = (value << 1) ^ (value >> 31);
        Self::put_u32(value, writer);
    }

    pub fn put_i64(value: i64, writer: &mut impl BufMut) {
        let value: u64 = unsafe { transmute(value) };
        let value = (value << 1) ^ (value >> 63);
        Self::put_u64(value, writer);
    }

    pub fn get_u8(reader: &mut impl Buf) -> Result<u8> {
        let remaining = reader.remaining();
        if remaining == 0 {
            return Err(VarintError::EndOfInput);
        }

        let head = reader.get_u8();

        if (head & 0x80) == 0 {
            Ok(head)
        } else if head == Self::INT8_CODE {
            if remaining < 2 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u8())
        } else {
            Err(VarintError::InvalidHeader { value: head })
        }
    }

    pub fn get_u16(reader: &mut impl Buf) -> Result<u16> {
        let remaining = reader.remaining();
        if remaining == 0 {
            return Err(VarintError::EndOfInput);
        }

        let head = reader.get_u8();

        if (head & 0x80) == 0 {
            Ok(head as u16)
        } else if head == Self::INT8_CODE {
            if remaining < 2 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u8() as u16)
        } else if head == Self::INT16_CODE {
            if remaining < 3 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u16_le())
        } else {
            Err(VarintError::InvalidHeader { value: head })
        }
    }

    pub fn get_u32(reader: &mut impl Buf) -> Result<u32> {
        let remaining = reader.remaining();
        if remaining == 0 {
            return Err(VarintError::EndOfInput);
        }

        let head = reader.get_u8();

        if (head & 0x80) == 0 {
            Ok(head as u32)
        } else if head == Self::INT8_CODE {
            if remaining < 2 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u8() as u32)
        } else if head == Self::INT16_CODE {
            if remaining < 3 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u16_le() as u32)
        } else if head == Self::INT32_CODE {
            if remaining < 5 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u32_le())
        } else {
            Err(VarintError::InvalidHeader { value: head })
        }
    }

    pub fn get_u64(reader: &mut impl Buf) -> Result<u64> {
        let remaining = reader.remaining();
        if remaining == 0 {
            return Err(VarintError::EndOfInput);
        }

        let head = reader.get_u8();

        if (head & 0x80) == 0 {
            Ok(head as u64)
        } else if head == Self::INT8_CODE {
            if remaining < 2 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u8() as u64)
        } else if head == Self::INT16_CODE {
            if remaining < 3 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u16_le() as u64)
        } else if head == Self::INT32_CODE {
            if remaining < 5 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u32_le() as u64)
        } else if head == Self::INT64_CODE {
            if remaining < 9 {
                return Err(VarintError::TooSmall { size: remaining });
            }
            Ok(reader.get_u64_le())
        } else {
            Err(VarintError::InvalidHeader { value: head })
        }
    }

    pub fn get_i8(reader: &mut impl Buf) -> Result<i8> {
        let value = Self::get_u8(reader)?;
        let value = (value << 7) ^ (value >> 1);
        let value: i8 = unsafe { transmute(value) };
        Ok(value)
    }

    pub fn get_i16(reader: &mut impl Buf) -> Result<i16> {
        let value = Self::get_u16(reader)?;
        let value = (value << 15) ^ (value >> 1);
        let value: i16 = unsafe { transmute(value) };
        Ok(value)
    }

    pub fn get_i32(reader: &mut impl Buf) -> Result<i32> {
        let value = Self::get_u32(reader)?;
        let value = (value << 31) ^ (value >> 1);
        let value: i32 = unsafe { transmute(value) };
        Ok(value)
    }

    pub fn get_i64(reader: &mut impl Buf) -> Result<i64> {
        let value = Self::get_u64(reader)?;
        let value = (value << 63) ^ (value >> 1);
        let value: i64 = unsafe { transmute(value) };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use testresult::TestResult;
    use tokio_util::bytes::BytesMut;

    use super::*;

    const INT8_CODE: u8 = 0x80;
    const INT16_CODE: u8 = 0x81;
    const INT32_CODE: u8 = 0x82;
    const INT64_CODE: u8 = 0x83;

    #[test]
    fn empty_data_get_test() -> TestResult {
        let buf = BytesMut::new();

        // 8
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u8(&mut buf), Err(VarintError::EndOfInput));
        }
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i8(&mut buf), Err(VarintError::EndOfInput));
        }

        // 16
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u16(&mut buf), Err(VarintError::EndOfInput));
        }
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i16(&mut buf), Err(VarintError::EndOfInput));
        }

        // 32
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u32(&mut buf), Err(VarintError::EndOfInput));
        }
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i32(&mut buf), Err(VarintError::EndOfInput));
        }

        // 64
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::EndOfInput));
        }
        {
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::EndOfInput));
        }

        Ok(())
    }

    #[test]
    fn broken_header_data_get_test() -> TestResult {
        // 8
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT16_CODE);

            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u8(&mut buf), Err(VarintError::InvalidHeader { value: INT16_CODE }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i8(&mut buf), Err(VarintError::InvalidHeader { value: INT16_CODE }));
            }
        }

        // 16
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT32_CODE);

            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u16(&mut buf), Err(VarintError::InvalidHeader { value: INT32_CODE }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i16(&mut buf), Err(VarintError::InvalidHeader { value: INT32_CODE }));
            }
        }

        // 32
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT64_CODE);

            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u32(&mut buf), Err(VarintError::InvalidHeader { value: INT64_CODE }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i32(&mut buf), Err(VarintError::InvalidHeader { value: INT64_CODE }));
            }
        }

        // 64
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT64_CODE + 1);

            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::InvalidHeader { value: INT64_CODE + 1 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::InvalidHeader { value: INT64_CODE + 1 }));
            }
        }

        Ok(())
    }

    #[test]
    fn broken_body_data_get_test() -> TestResult {
        // INT8_CODE
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT8_CODE);

            // 8
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u8(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i8(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }

            // 16
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u16(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i16(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }

            // 32
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u32(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i32(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }

            // 64
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::TooSmall { size: 1 }));
            }
        }

        // INT16_CODE
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT16_CODE);
            buf.put_bytes(0x00, 1);

            // 16
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u16(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i16(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }

            // 32
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u32(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i32(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }

            // 64
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::TooSmall { size: 2 }));
            }
        }

        // INT32_CODE
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT32_CODE);
            buf.put_bytes(0x00, 3);

            // 32
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u32(&mut buf), Err(VarintError::TooSmall { size: 4 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i32(&mut buf), Err(VarintError::TooSmall { size: 4 }));
            }

            // 64
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::TooSmall { size: 4 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::TooSmall { size: 4 }));
            }
        }

        // INT64_CODE
        {
            let mut buf = BytesMut::new();
            buf.put_u8(INT64_CODE);
            buf.put_bytes(0x00, 7);

            // 64
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_u64(&mut buf), Err(VarintError::TooSmall { size: 8 }));
            }
            {
                let mut buf = buf.clone().freeze();
                assert_eq!(Varint::get_i64(&mut buf), Err(VarintError::TooSmall { size: 8 }));
            }
        }

        Ok(())
    }

    #[test]
    fn random_test() -> TestResult {
        let mut rng = ChaCha20Rng::from_seed(Default::default());

        // 8
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_u8(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u8(&mut buf)?, v);
        }
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_i8(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i8(&mut buf)?, v);
        }

        // 16
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_u16(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u16(&mut buf)?, v);
        }
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_i16(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i16(&mut buf)?, v);
        }

        // 32
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_u32(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u32(&mut buf)?, v);
        }
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_i32(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i32(&mut buf)?, v);
        }

        // 64
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_u64(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_u64(&mut buf)?, v);
        }
        for _ in 0..32 {
            let v = rng.r#gen();
            let mut buf = BytesMut::new();
            Varint::put_i64(v, &mut buf);
            let mut buf = buf.clone().freeze();
            assert_eq!(Varint::get_i64(&mut buf)?, v);
        }

        Ok(())
    }
}
