use tokio_util::bytes::{Buf, Bytes};

use crate::{
    prelude::*,
    primitive::{Timestamp64, Timestamp96, Varint},
};

pub struct RocketMessageReader<'a> {
    reader: &'a mut Bytes,
}

impl<'a> RocketMessageReader<'a> {
    pub fn new(reader: &'a mut Bytes) -> Self {
        RocketMessageReader { reader }
    }

    pub fn get_bytes(&mut self, limit: usize) -> Result<Vec<u8>> {
        let length = self.get_u32()? as usize;
        if length > limit {
            return Err(Error::builder()
                .kind(ErrorKind::TooLarge)
                .message(format!("length exceeded limit: {length} > {limit}"))
                .build());
        }

        if length == 0 {
            return Ok(Vec::new());
        }

        if self.reader.remaining() < length {
            return Err(Error::builder().kind(ErrorKind::EndOfStream).build());
        }

        let mut result = vec![0u8; length];
        self.reader.copy_to_slice(&mut result);
        Ok(result)
    }

    pub fn get_string(&mut self, limit: usize) -> Result<String> {
        let result = self.get_bytes(limit)?;
        String::from_utf8(result).map_err(|_| Error::builder().kind(ErrorKind::InvalidFormat).message("invalid utf-8").build())
    }

    pub fn get_timestamp64(&mut self) -> Result<Timestamp64> {
        let seconds = self.get_i64()?;
        Ok(Timestamp64::new(seconds))
    }

    pub fn get_timestamp96(&mut self) -> Result<Timestamp96> {
        let seconds = self.get_i64()?;
        let nanos = self.get_u32()?;
        Ok(Timestamp96::new(seconds, nanos))
    }

    pub fn get_bool(&mut self) -> Result<bool> {
        let result = self.get_u64()?;
        Ok(result != 0)
    }

    pub fn get_u8(&mut self) -> Result<u8> {
        Ok(Varint::get_u8(self.reader)?)
    }

    pub fn get_u16(&mut self) -> Result<u16> {
        Ok(Varint::get_u16(self.reader)?)
    }

    pub fn get_u32(&mut self) -> Result<u32> {
        Ok(Varint::get_u32(self.reader)?)
    }

    pub fn get_u64(&mut self) -> Result<u64> {
        Ok(Varint::get_u64(self.reader)?)
    }

    pub fn get_i8(&mut self) -> Result<i8> {
        Ok(Varint::get_i8(self.reader)?)
    }

    pub fn get_i16(&mut self) -> Result<i16> {
        Ok(Varint::get_i16(self.reader)?)
    }

    pub fn get_i32(&mut self) -> Result<i32> {
        Ok(Varint::get_i32(self.reader)?)
    }

    pub fn get_i64(&mut self) -> Result<i64> {
        Ok(Varint::get_i64(self.reader)?)
    }

    pub fn get_f32(&mut self) -> Result<f32> {
        const SIZE: usize = 4;

        if self.reader.remaining() < SIZE {
            return Err(Error::builder().kind(ErrorKind::EndOfStream).build());
        }

        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f32::from_le_bytes(buffer))
    }

    pub fn get_f64(&mut self) -> Result<f64> {
        const SIZE: usize = 8;

        if self.reader.remaining() < SIZE {
            return Err(Error::builder().kind(ErrorKind::EndOfStream).build());
        }

        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f64::from_le_bytes(buffer))
    }
}

#[cfg(test)]
mod tests {

    use testresult::TestResult;

    use super::*;

    #[test]
    fn get_bytes_err_too_large_test() -> TestResult {
        let mut bytes = Bytes::from(hex::decode("02")?);
        let mut reader = RocketMessageReader::new(&mut bytes);

        assert!(reader.get_bytes(1).is_err_and(|x| *x.kind() == ErrorKind::TooLarge));

        Ok(())
    }

    #[test]
    fn get_bytes_err_end_of_input_test() -> TestResult {
        let mut bytes = Bytes::from(hex::decode("01")?);
        let mut reader = RocketMessageReader::new(&mut bytes);

        assert!(reader.get_bytes(1).is_err_and(|x| *x.kind() == ErrorKind::EndOfStream));

        Ok(())
    }
}
