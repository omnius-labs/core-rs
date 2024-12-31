use tokio_util::bytes::{Buf, Bytes};

use crate::{Timestamp64, Timestamp96, Varint};

pub struct RocketMessageReader<'a> {
    reader: &'a mut Bytes,
}

impl<'a> RocketMessageReader<'a> {
    pub fn new(reader: &'a mut Bytes) -> Self {
        RocketMessageReader { reader }
    }

    pub fn get_bytes(&mut self, limit: usize) -> Result<Vec<u8>, super::RocketMessageError> {
        let length = self.get_u32()?;
        if length > limit as u32 {
            return Err(super::RocketMessageError::TooLarge);
        }

        if length == 0 {
            return Ok(Vec::new());
        }

        if self.reader.remaining() < length as usize {
            return Err(super::RocketMessageError::EndOfInput);
        }

        let mut result = vec![0u8; length as usize];
        self.reader.copy_to_slice(&mut result);
        Ok(result)
    }

    pub fn get_string(&mut self, limit: usize) -> Result<String, super::RocketMessageError> {
        let result = self.get_bytes(limit)?;
        String::from_utf8(result).map_err(|_| super::RocketMessageError::InvalidUtf8)
    }

    pub fn get_timestamp64(&mut self) -> Result<Timestamp64, super::RocketMessageError> {
        let seconds = self.get_i64()?;
        Ok(Timestamp64::new(seconds))
    }

    pub fn get_timestamp96(&mut self) -> Result<Timestamp96, super::RocketMessageError> {
        let seconds = self.get_i64()?;
        let nanos = self.get_u32()?;
        Ok(Timestamp96::new(seconds, nanos))
    }

    pub fn get_bool(&mut self) -> Result<bool, super::RocketMessageError> {
        let result = self.get_u64()?;
        Ok(result != 0)
    }

    pub fn get_u8(&mut self) -> Result<u8, super::RocketMessageError> {
        Varint::get_u8(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_u16(&mut self) -> Result<u16, super::RocketMessageError> {
        Varint::get_u16(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_u32(&mut self) -> Result<u32, super::RocketMessageError> {
        Varint::get_u32(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_u64(&mut self) -> Result<u64, super::RocketMessageError> {
        Varint::get_u64(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_i8(&mut self) -> Result<i8, super::RocketMessageError> {
        Varint::get_i8(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_i16(&mut self) -> Result<i16, super::RocketMessageError> {
        Varint::get_i16(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_i32(&mut self) -> Result<i32, super::RocketMessageError> {
        Varint::get_i32(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_i64(&mut self) -> Result<i64, super::RocketMessageError> {
        Varint::get_i64(self.reader).map_err(super::RocketMessageError::VarintError)
    }

    pub fn get_f32(&mut self) -> Result<f32, super::RocketMessageError> {
        const SIZE: usize = 4;

        if self.reader.remaining() < SIZE {
            return Err(super::RocketMessageError::EndOfInput);
        }

        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f32::from_le_bytes(buffer))
    }

    pub fn get_f64(&mut self) -> Result<f64, super::RocketMessageError> {
        const SIZE: usize = 8;

        if self.reader.remaining() < SIZE {
            return Err(super::RocketMessageError::EndOfInput);
        }

        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f64::from_le_bytes(buffer))
    }
}

#[cfg(test)]
mod tests {

    use testresult::TestResult;

    use crate::RocketMessageError;

    use super::*;

    #[test]
    fn get_bytes_err_too_large_test() -> TestResult {
        let mut bytes = Bytes::from(hex::decode("02")?);
        let mut reader = RocketMessageReader::new(&mut bytes);

        assert!(reader
            .get_bytes(1)
            .is_err_and(|x| x == RocketMessageError::TooLarge));

        Ok(())
    }

    #[test]
    fn get_bytes_err_end_of_input_test() -> TestResult {
        let mut bytes = Bytes::from(hex::decode("01")?);
        let mut reader = RocketMessageReader::new(&mut bytes);

        assert!(reader
            .get_bytes(1)
            .is_err_and(|x| x == RocketMessageError::EndOfInput));

        Ok(())
    }
}
