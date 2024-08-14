use std::error::Error;
use tokio_util::bytes::{Buf, Bytes};

use crate::{FormatError, Timestamp64, Timestamp96, Varint};

pub struct RocketMessageReader<'a> {
    reader: &'a mut Bytes,
}

impl<'a> RocketMessageReader<'a> {
    pub fn new(reader: &'a mut Bytes) -> Self {
        RocketMessageReader { reader }
    }

    pub fn get_bytes(&mut self, limit: usize) -> Result<Bytes, Box<dyn Error>> {
        let length = self.get_u32()?;
        if length > limit as u32 {
            return Err(Box::new(FormatError));
        }

        if length == 0 {
            return Ok(Bytes::new());
        }

        let mut result = vec![0u8; length as usize];
        self.reader.copy_to_slice(&mut result);
        Ok(Bytes::from(result))
    }

    pub fn get_string(&mut self, limit: usize) -> Result<String, Box<dyn Error>> {
        let length = self.get_u32()?;
        if length > limit as u32 {
            return Err(Box::new(FormatError));
        }

        if length == 0 {
            return Ok(String::new());
        }

        let mut result = vec![0u8; length as usize];
        self.reader.copy_to_slice(&mut result);
        Ok(String::from_utf8(result)?)
    }

    pub fn get_timestamp64(&mut self) -> Result<Timestamp64, Box<dyn Error>> {
        let seconds = self.get_i64()?;
        Ok(Timestamp64::new(seconds))
    }

    pub fn get_timestamp96(&mut self) -> Result<Timestamp96, Box<dyn Error>> {
        let seconds = self.get_i64()?;
        let nanos = self.get_u32()?;
        Ok(Timestamp96::new(seconds, nanos))
    }

    pub fn get_bool(&mut self) -> Result<bool, Box<dyn Error>> {
        let result = self.get_u64()?;
        Ok(result != 0)
    }

    pub fn get_u8(&mut self) -> Result<u8, Box<dyn Error>> {
        Varint::get_u8(self.reader)
    }

    pub fn get_u16(&mut self) -> Result<u16, Box<dyn Error>> {
        Varint::get_u16(self.reader)
    }

    pub fn get_u32(&mut self) -> Result<u32, Box<dyn Error>> {
        Varint::get_u32(self.reader)
    }

    pub fn get_u64(&mut self) -> Result<u64, Box<dyn Error>> {
        Varint::get_u64(self.reader)
    }

    pub fn get_i8(&mut self) -> Result<i8, Box<dyn Error>> {
        Varint::get_i8(self.reader)
    }

    pub fn get_i16(&mut self) -> Result<i16, Box<dyn Error>> {
        Varint::get_i16(self.reader)
    }

    pub fn get_i32(&mut self) -> Result<i32, Box<dyn Error>> {
        Varint::get_i32(self.reader)
    }

    pub fn get_i64(&mut self) -> Result<i64, Box<dyn Error>> {
        Varint::get_i64(self.reader)
    }

    pub fn get_f32(&mut self) -> Result<f32, Box<dyn Error>> {
        const SIZE: usize = 4;
        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f32::from_le_bytes(buffer))
    }

    pub fn get_f64(&mut self) -> Result<f64, Box<dyn Error>> {
        const SIZE: usize = 8;
        let mut buffer = [0u8; SIZE];
        self.reader.copy_to_slice(&mut buffer);
        Ok(f64::from_le_bytes(buffer))
    }
}
