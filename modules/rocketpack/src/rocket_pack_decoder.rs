use thiserror::Error;

use crate::{FieldType, RocketPackStruct};

type Result<T> = std::result::Result<T, RocketPackDecoderError>;

// https://cborbook.com/part_1/practical_introduction_to_cbor.html

#[derive(Error, Debug)]
pub enum RocketPackDecoderError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("mismatch field type at position  (position: {position}, type: {field_type})")]
    MismatchFieldType { position: usize, field_type: FieldType },
    #[error("length overflow (position: {position})")]
    LengthOverflow { position: usize },
    #[error("string is not valid UTF-8 (position: {position}, error: {error})")]
    Utf8 { position: usize, error: std::str::Utf8Error },
    #[error("other decode error: {0}")]
    Other(&'static str),
}

pub trait RocketPackDecoder {
    fn remaining(&self) -> usize;
    fn position(&self) -> usize;
    fn current_type(&self) -> Result<FieldType>;
    fn read_bool(&mut self) -> Result<bool>;
    fn read_u8(&mut self) -> Result<u8>;
    fn read_u16(&mut self) -> Result<u16>;
    fn read_u32(&mut self) -> Result<u32>;
    fn read_u64(&mut self) -> Result<u64>;
    fn read_i8(&mut self) -> Result<i8>;
    fn read_i16(&mut self) -> Result<i16>;
    fn read_i32(&mut self) -> Result<i32>;
    fn read_i64(&mut self) -> Result<i64>;
    fn read_f32(&mut self) -> Result<f32>;
    fn read_f64(&mut self) -> Result<f64>;
    fn read_bytes(&mut self) -> Result<&[u8]>;
    fn read_bytes_vec(&mut self) -> Result<Vec<u8>>;
    fn read_string(&mut self) -> Result<String>;
    fn read_array(&mut self) -> Result<u64>;
    fn read_map(&mut self) -> Result<u64>;
    fn read_null(&mut self) -> Result<()>;
    fn read_struct<T: RocketPackStruct>(&mut self) -> Result<T>;
    fn skip_field(&mut self) -> Result<()>;
}

pub struct RocketPackBytesDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> RocketPackBytesDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl<'a> RocketPackDecoder for RocketPackBytesDecoder<'a> {
    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn position(&self) -> usize {
        self.pos
    }

    fn current_type(&self) -> Result<FieldType> {
        let (major, info) = self.decompose(self.current_raw_byte()?);
        self.type_of(major, info)
    }

    fn read_bool(&mut self) -> Result<bool> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        Ok(match (major, info) {
            (7, 20) => false,
            (7, 21) => true,
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_u8(&mut self) -> Result<u8> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        Ok(match (major, info) {
            (0, 0..=23) => info,
            (0, 24) => u8::from_be_bytes(self.read_raw_fixed_bytes()?),
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_u16(&mut self) -> Result<u16> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        Ok(match (major, info) {
            (0, 0..=23) => info as u16,
            (0, 24) => u8::from_be_bytes(self.read_raw_fixed_bytes()?) as u16,
            (0, 25) => u16::from_be_bytes(self.read_raw_fixed_bytes()?),
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_u32(&mut self) -> Result<u32> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        Ok(match (major, info) {
            (0, 0..=23) => info as u32,
            (0, 24) => u8::from_be_bytes(self.read_raw_fixed_bytes()?) as u32,
            (0, 25) => u16::from_be_bytes(self.read_raw_fixed_bytes()?) as u32,
            (0, 26) => u32::from_be_bytes(self.read_raw_fixed_bytes()?),
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_u64(&mut self) -> Result<u64> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        Ok(match (major, info) {
            (0, 0..=23) => info as u64,
            (0, 24) => u8::from_be_bytes(self.read_raw_fixed_bytes()?) as u64,
            (0, 25) => u16::from_be_bytes(self.read_raw_fixed_bytes()?) as u64,
            (0, 26) => u32::from_be_bytes(self.read_raw_fixed_bytes()?) as u64,
            (0, 27) => u64::from_be_bytes(self.read_raw_fixed_bytes()?),
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_i8(&mut self) -> Result<i8> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        match (major, info) {
            (0, 0..=23) => return Ok(info as i8),
            (0, 24) => return Ok(u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i8),
            (1, 0..=23) => return Ok(-1 - (info as i8)),
            (1, 24..=28) => {
                // Determine the smallest signed integer type the value fits in.
                if (self.current_raw_byte()? & 0x80) != 0x80 {
                    #[allow(clippy::single_match)]
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i8)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_i16(&mut self) -> Result<i16> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        match (major, info) {
            (0, 0..=23) => return Ok(info as i16),
            (0, 24) => return Ok(u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i16),
            (0, 25) => return Ok(u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i16),
            (1, 0..=23) => return Ok(-1 - (info as i16)),
            (1, 24..=28) => {
                // Determine the smallest signed integer type the value fits in.
                if (self.current_raw_byte()? & 0x80) != 0x80 {
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i16)),
                        25 => return Ok(-1 - (u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i16)),
                        _ => {}
                    }
                } else {
                    #[allow(clippy::single_match)]
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i16)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_i32(&mut self) -> Result<i32> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        match (major, info) {
            (0, 0..=23) => return Ok(info as i32),
            (0, 24) => return Ok(u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i32),
            (0, 25) => return Ok(u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i32),
            (0, 26) => return Ok(u32::from_be_bytes(self.read_raw_fixed_bytes()?) as i32),
            (1, 0..=23) => return Ok(-1 - (info as i32)),
            (1, 24..=28) => {
                // Determine the smallest signed integer type the value fits in.
                if (self.current_raw_byte()? & 0x80) != 0x80 {
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i32)),
                        25 => return Ok(-1 - (u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i32)),
                        26 => return Ok(-1 - (u32::from_be_bytes(self.read_raw_fixed_bytes()?) as i32)),
                        _ => {}
                    }
                } else {
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i32)),
                        25 => return Ok(-1 - (u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i32)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_i64(&mut self) -> Result<i64> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        match (major, info) {
            (0, 0..=23) => return Ok(info as i64),
            (0, 24) => return Ok(u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i64),
            (0, 25) => return Ok(u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i64),
            (0, 26) => return Ok(u32::from_be_bytes(self.read_raw_fixed_bytes()?) as i64),
            (0, 27) => return Ok(u64::from_be_bytes(self.read_raw_fixed_bytes()?) as i64),
            (1, 0..=23) => return Ok(-1 - (info as i64)),
            (1, 24..=28) => {
                // Determine the smallest signed integer type the value fits in.
                if (self.current_raw_byte()? & 0x80) != 0x80 {
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        25 => return Ok(-1 - (u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        26 => return Ok(-1 - (u32::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        27 => return Ok(-1 - (u64::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        _ => {}
                    }
                } else {
                    match info {
                        24 => return Ok(-1 - (u8::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        25 => return Ok(-1 - (u16::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        26 => return Ok(-1 - (u32::from_be_bytes(self.read_raw_fixed_bytes()?) as i64)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_f32(&mut self) -> Result<f32> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if (major, info) == (7, 26) {
            return Ok(f32::from_be_bytes(self.read_raw_fixed_bytes()?));
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_f64(&mut self) -> Result<f64> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if (major, info) == (7, 27) {
            return Ok(f64::from_be_bytes(self.read_raw_fixed_bytes()?));
        }

        Err(RocketPackDecoderError::MismatchFieldType {
            position: p,
            field_type: self.type_of(major, info)?,
        })
    }

    fn read_bytes(&mut self) -> Result<&'a [u8]> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if major != 2 {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        }

        let Some(len) = self.read_raw_len(info)? else {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        };
        let len: usize = len.try_into().map_err(|_| RocketPackDecoderError::LengthOverflow { position: p })?;
        self.read_raw_bytes(len)
    }

    fn read_bytes_vec(&mut self) -> Result<Vec<u8>> {
        Ok(self.read_bytes()?.to_vec())
    }

    fn read_string(&mut self) -> Result<String> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if major != 3 {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        }

        let Some(len) = self.read_raw_len(info)? else {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        };
        let len: usize = len.try_into().map_err(|_| RocketPackDecoderError::LengthOverflow { position: p })?;
        let bytes = self.read_raw_bytes(len)?;

        std::str::from_utf8(bytes)
            .map(|n| n.to_owned())
            .map_err(|e| RocketPackDecoderError::Utf8 { position: p, error: e })
    }

    fn read_array(&mut self) -> Result<u64> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if major != 4 {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        }

        let Some(len) = self.read_raw_len(info)? else {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        };

        Ok(len)
    }

    fn read_map(&mut self) -> Result<u64> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        if major != 5 {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        }

        let Some(len) = self.read_raw_len(info)? else {
            return Err(RocketPackDecoderError::MismatchFieldType {
                position: p,
                field_type: self.type_of(major, info)?,
            });
        };

        Ok(len)
    }

    fn read_null(&mut self) -> Result<()> {
        let p = self.pos;
        let v = self.read_raw_byte()?;
        let (major, info) = self.decompose(v);

        #[allow(clippy::unit_arg)]
        Ok(match (major, info) {
            (7, 22) => (),
            _ => {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            }
        })
    }

    fn read_struct<T: RocketPackStruct>(&mut self) -> Result<T> {
        T::unpack(self)
    }

    fn skip_field(&mut self) -> Result<()> {
        let mut remain: u64 = 1;

        while remain > 0 {
            let p = self.pos;
            let v = self.read_raw_byte()?;
            let (major, info) = self.decompose(v);

            let len = match major {
                0 | 1 => match info {
                    0..=23 => Some(0),
                    24 => Some(1),
                    25 => Some(2),
                    26 => Some(4),
                    27 => Some(8),
                    28 => Some(16),
                    _ => None,
                },
                2 | 3 => self.read_raw_len(info)?,
                4 => match self.read_raw_len(info)? {
                    Some(count) => {
                        remain = remain.checked_add(count).ok_or(RocketPackDecoderError::LengthOverflow { position: p })?;
                        Some(0)
                    }
                    _ => None,
                },
                5 => match self.read_raw_len(info)? {
                    Some(count) => {
                        let count = count.checked_mul(2).ok_or(RocketPackDecoderError::LengthOverflow { position: p })?;
                        remain = remain.checked_add(count).ok_or(RocketPackDecoderError::LengthOverflow { position: p })?;
                        Some(0)
                    }
                    _ => None,
                },
                7 => match info {
                    20 | 21 => Some(0),
                    25 => Some(2),
                    26 => Some(4),
                    27 => Some(8),
                    _ => None,
                },
                _ => None,
            };

            let Some(len) = len else {
                return Err(RocketPackDecoderError::MismatchFieldType {
                    position: p,
                    field_type: self.type_of(major, info)?,
                });
            };
            let len: usize = len.try_into().map_err(|_| RocketPackDecoderError::LengthOverflow { position: p })?;
            self.skip_raw_bytes(len)?;

            remain -= 1;
        }

        Ok(())
    }
}

impl<'a> RocketPackBytesDecoder<'a> {
    fn is_eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    #[inline]
    fn decompose(&self, v: u8) -> (u8, u8) {
        let major = v >> 5; // major type
        let info = v & 0b0001_1111; // additional information
        (major, info)
    }

    // major: major type
    // info: additional information
    fn type_of(&self, major: u8, info: u8) -> Result<FieldType> {
        match (major, info) {
            (0, 0..=23) => return Ok(FieldType::U8),
            (0, 24) => return Ok(FieldType::U8),
            (0, 25) => return Ok(FieldType::U16),
            (0, 26) => return Ok(FieldType::U32),
            (0, 27) => return Ok(FieldType::U64),
            (1, 0..=23) => return Ok(FieldType::U8),
            (1, 24..=28) => {
                // Determine the smallest signed integer type the value fits in.
                if (self.peek_raw_byte()? & 0x80) != 0x80 {
                    match info {
                        24 => return Ok(FieldType::I8),
                        25 => return Ok(FieldType::I16),
                        26 => return Ok(FieldType::I32),
                        27 => return Ok(FieldType::I64),
                        _ => {}
                    }
                } else {
                    match info {
                        24 => return Ok(FieldType::I16),
                        25 => return Ok(FieldType::I32),
                        26 => return Ok(FieldType::I64),
                        _ => {}
                    }
                }
            }
            (2, _) => return Ok(FieldType::Bytes),
            (3, _) => return Ok(FieldType::String),
            (4, _) => return Ok(FieldType::Array),
            (5, _) => return Ok(FieldType::Map),
            (7, 20..=21) => return Ok(FieldType::Bool),
            (7, 25) => return Ok(FieldType::F16),
            (7, 26) => return Ok(FieldType::F32),
            (7, 27) => return Ok(FieldType::F64),
            _ => {}
        }

        Ok(FieldType::Unknown { major, info })
    }

    pub(crate) fn read_raw_len(&mut self, info: u8) -> Result<Option<u64>> {
        Ok(match info {
            0..=23 => Some(info as u64),
            24 => Some(u8::from_be_bytes(self.read_raw_fixed_bytes()?) as u64),
            25 => Some(u16::from_be_bytes(self.read_raw_fixed_bytes()?) as u64),
            26 => Some(u32::from_be_bytes(self.read_raw_fixed_bytes()?) as u64),
            27 => Some(u64::from_be_bytes(self.read_raw_fixed_bytes()?)),
            _ => None,
        })
    }

    fn current_raw_byte(&self) -> Result<u8> {
        if self.is_eof() {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        Ok(self.buf[self.pos])
    }

    fn peek_raw_byte(&self) -> Result<u8> {
        if self.remaining() < 2 {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        Ok(self.buf[self.pos + 1])
    }

    fn read_raw_byte(&mut self) -> Result<u8> {
        if self.remaining() < 1 {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_raw_fixed_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.remaining() < N {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        let mut out = [0u8; N];
        let end = self.pos + N;
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
        Ok(out)
    }

    fn read_raw_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.remaining() < len {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        let end = self.pos + len;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn skip_raw_bytes(&mut self, len: usize) -> Result<()> {
        if self.remaining() < len {
            return Err(RocketPackDecoderError::UnexpectedEof);
        }
        self.pos += len;
        Ok(())
    }
}
