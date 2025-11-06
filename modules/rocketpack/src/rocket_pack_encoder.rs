use std::io::Write;

use thiserror::Error;

use crate::RocketPackStruct;

type Result<T> = std::result::Result<T, RocketPackEncoderError>;

#[derive(Error, Debug)]
pub enum RocketPackEncoderError {
    #[error("I/O error occurred")]
    IoError(#[from] std::io::Error),
    #[error("length overflow")]
    LengthOverflow { len: usize },
}

pub trait RocketPackEncoder {
    fn write_bool(&mut self, value: bool) -> Result<()>;
    fn write_u8(&mut self, value: u8) -> Result<()>;
    fn write_u16(&mut self, value: u16) -> Result<()>;
    fn write_u32(&mut self, value: u32) -> Result<()>;
    fn write_u64(&mut self, value: u64) -> Result<()>;
    fn write_i8(&mut self, value: i8) -> Result<()>;
    fn write_i16(&mut self, value: i16) -> Result<()>;
    fn write_i32(&mut self, value: i32) -> Result<()>;
    fn write_i64(&mut self, value: i64) -> Result<()>;
    fn write_f32(&mut self, value: f32) -> Result<()>;
    fn write_f64(&mut self, value: f64) -> Result<()>;
    fn write_bytes(&mut self, value: &[u8]) -> Result<()>;
    fn write_string(&mut self, value: &str) -> Result<()>;
    fn write_array(&mut self, len: usize) -> Result<()>;
    fn write_map(&mut self, len: usize) -> Result<()>;
    fn write_struct<T: RocketPackStruct>(&mut self, value: &T) -> Result<()>;
}

pub struct RocketPackBytesEncoder<W: Write> {
    writer: W,
}

impl<W: Write> RocketPackBytesEncoder<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> RocketPackEncoder for RocketPackBytesEncoder<W> {
    fn write_bool(&mut self, value: bool) -> Result<()> {
        self.write_raw_bytes(&[self.compose(7, if !value { 20 } else { 21 })])
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        if value <= 23 {
            self.write_raw_bytes(&[self.compose(0, value)])?;
        } else {
            self.write_raw_bytes(&[self.compose(0, 24), value])?;
        }
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<()> {
        if value <= 23 {
            self.write_raw_bytes(&[self.compose(0, value as u8)])?;
        } else if value <= u8::MAX as u16 {
            self.write_raw_bytes(&[self.compose(0, 24), value as u8])?;
        } else {
            self.write_raw_bytes(&[self.compose(0, 25)])?;
            self.write_raw_bytes(value.to_be_bytes().as_slice())?;
        }
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        if value <= 23 {
            self.write_raw_bytes(&[self.compose(0, value as u8)])?;
        } else if value <= u8::MAX as u32 {
            self.write_raw_bytes(&[self.compose(0, 24), value as u8])?;
        } else if value <= u16::MAX as u32 {
            self.write_raw_bytes(&[self.compose(0, 25)])?;
            self.write_raw_bytes((value as u16).to_be_bytes().as_slice())?;
        } else {
            self.write_raw_bytes(&[self.compose(0, 26)])?;
            self.write_raw_bytes(value.to_be_bytes().as_slice())?;
        }
        Ok(())
    }

    fn write_u64(&mut self, value: u64) -> Result<()> {
        if value <= 23 {
            self.write_raw_bytes(&[self.compose(0, value as u8)])?;
        } else if value <= u8::MAX as u64 {
            self.write_raw_bytes(&[self.compose(0, 24), value as u8])?;
        } else if value <= u16::MAX as u64 {
            self.write_raw_bytes(&[self.compose(0, 25)])?;
            self.write_raw_bytes((value as u16).to_be_bytes().as_slice())?;
        } else if value <= u32::MAX as u64 {
            self.write_raw_bytes(&[self.compose(0, 26)])?;
            self.write_raw_bytes((value as u32).to_be_bytes().as_slice())?;
        } else {
            self.write_raw_bytes(&[self.compose(0, 27)])?;
            self.write_raw_bytes(value.to_be_bytes().as_slice())?;
        }
        Ok(())
    }

    fn write_i8(&mut self, value: i8) -> Result<()> {
        if value >= 0 {
            self.write_u8(value as u8)?;
        } else {
            let v = (-1 - value) as u8;
            if v <= 23 {
                self.write_raw_bytes(&[self.compose(1, v)])?;
            } else {
                self.write_raw_bytes(&[self.compose(1, 24), v])?;
            }
        }
        Ok(())
    }

    fn write_i16(&mut self, value: i16) -> Result<()> {
        if value >= 0 {
            self.write_u16(value as u16)?;
        } else {
            let v = (-1 - value) as u16;
            if v <= 23 {
                self.write_raw_bytes(&[self.compose(1, v as u8)])?;
            } else if v <= u8::MAX as u16 {
                self.write_raw_bytes(&[self.compose(1, 24), v as u8])?;
            } else {
                self.write_raw_bytes(&[self.compose(1, 25)])?;
                self.write_raw_bytes(v.to_be_bytes().as_slice())?;
            }
        }
        Ok(())
    }

    fn write_i32(&mut self, value: i32) -> Result<()> {
        if value >= 0 {
            self.write_u32(value as u32)?;
        } else {
            let v = (-1 - value) as u32;
            if v <= 23 {
                self.write_raw_bytes(&[self.compose(1, v as u8)])?;
            } else if v <= u8::MAX as u32 {
                self.write_raw_bytes(&[self.compose(1, 24), v as u8])?;
            } else if v <= u16::MAX as u32 {
                self.write_raw_bytes(&[self.compose(1, 25)])?;
                self.write_raw_bytes((v as u16).to_be_bytes().as_slice())?;
            } else {
                self.write_raw_bytes(&[self.compose(1, 26)])?;
                self.write_raw_bytes(v.to_be_bytes().as_slice())?;
            }
        }
        Ok(())
    }

    fn write_i64(&mut self, value: i64) -> Result<()> {
        if value >= 0 {
            self.write_u64(value as u64)?;
        } else {
            let v = (-1 - value) as u64;
            if v <= 23 {
                self.write_raw_bytes(&[self.compose(1, v as u8)])?;
            } else if v <= u8::MAX as u64 {
                self.write_raw_bytes(&[self.compose(1, 24), v as u8])?;
            } else if v <= u16::MAX as u64 {
                self.write_raw_bytes(&[self.compose(1, 25)])?;
                self.write_raw_bytes((v as u16).to_be_bytes().as_slice())?;
            } else if v <= u32::MAX as u64 {
                self.write_raw_bytes(&[self.compose(1, 26)])?;
                self.write_raw_bytes((v as u32).to_be_bytes().as_slice())?;
            } else {
                self.write_raw_bytes(&[self.compose(1, 27)])?;
                self.write_raw_bytes(v.to_be_bytes().as_slice())?;
            }
        }
        Ok(())
    }

    fn write_f32(&mut self, value: f32) -> Result<()> {
        self.write_raw_bytes(&[self.compose(7, 26)])?;
        self.write_raw_bytes(value.to_be_bytes().as_slice())
    }

    fn write_f64(&mut self, value: f64) -> Result<()> {
        self.write_raw_bytes(&[self.compose(7, 27)])?;
        self.write_raw_bytes(value.to_be_bytes().as_slice())
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        self.write_raw_len(2, value.len())?;
        self.write_raw_bytes(value)
    }

    fn write_string(&mut self, value: &str) -> Result<()> {
        self.write_raw_len(3, value.len())?;
        self.write_raw_bytes(value.as_bytes())
    }

    fn write_array(&mut self, len: usize) -> Result<()> {
        self.write_raw_len(4, len)
    }

    fn write_map(&mut self, len: usize) -> Result<()> {
        self.write_raw_len(5, len)
    }

    fn write_struct<T: RocketPackStruct>(&mut self, value: &T) -> Result<()> {
        T::pack(self, value)
    }
}

impl<W: Write> RocketPackBytesEncoder<W> {
    #[inline]
    fn compose(&self, major: u8, info: u8) -> u8 {
        (major << 5) | (info & 0b0001_1111)
    }

    pub(crate) fn write_raw_len(&mut self, major: u8, len: usize) -> Result<()> {
        let len: u64 = len.try_into().map_err(|_| RocketPackEncoderError::LengthOverflow { len })?;

        if len <= 23 {
            self.write_raw_bytes(&[self.compose(major, len as u8)])?;
        } else if len <= u8::MAX as u64 {
            self.write_raw_bytes(&[self.compose(major, 24)])?;
            self.write_raw_bytes((len as u8).to_be_bytes().as_slice())?;
        } else if len <= u16::MAX as u64 {
            self.write_raw_bytes(&[self.compose(major, 25)])?;
            self.write_raw_bytes((len as u16).to_be_bytes().as_slice())?;
        } else if len <= u32::MAX as u64 {
            self.write_raw_bytes(&[self.compose(major, 26)])?;
            self.write_raw_bytes((len as u32).to_be_bytes().as_slice())?;
        } else {
            self.write_raw_bytes(&[self.compose(major, 27)])?;
            self.write_raw_bytes(len.to_be_bytes().as_slice())?;
        }
        Ok(())
    }

    fn write_raw_bytes(&mut self, value: &[u8]) -> Result<()> {
        self.writer.write_all(value).map_err(RocketPackEncoderError::IoError)
    }
}
