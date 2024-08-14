use std::error::Error;

use tokio_util::bytes::{Buf, BufMut};

use crate::FormatError;

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
            writer.put_u16(value);
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
            writer.put_u16(value as u16);
        } else {
            writer.put_u8(Self::INT32_CODE);
            writer.put_u32(value);
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
            writer.put_u16(value as u16);
        } else if value <= u32::MAX as u64 {
            writer.put_u8(Self::INT32_CODE);
            writer.put_u32(value as u32);
        } else {
            writer.put_u8(Self::INT64_CODE);
            writer.put_u64(value);
        }
    }

    pub fn put_i8(value: i8, writer: &mut impl BufMut) {
        Self::put_u8(((value << 1) ^ (value >> 7)) as u8, writer);
    }

    pub fn put_i16(value: i16, writer: &mut impl BufMut) {
        Self::put_u16(((value << 1) ^ (value >> 15)) as u16, writer);
    }

    pub fn put_i32(value: i32, writer: &mut impl BufMut) {
        Self::put_u32(((value << 1) ^ (value >> 31)) as u32, writer);
    }

    pub fn put_i64(value: i64, writer: &mut impl BufMut) {
        Self::put_u64(((value << 1) ^ (value >> 63)) as u64, writer);
    }

    pub fn get_u8(reader: &mut impl Buf) -> Result<u8, Box<dyn Error>> {
        let head = reader.get_u8();
        if reader.remaining() >= 1 && (head & 0x80) == 0 {
            Ok(head)
        } else if reader.remaining() >= 2 && head == Self::INT8_CODE {
            Ok(reader.get_u8())
        } else {
            Err(Box::new(FormatError))
        }
    }

    pub fn get_u16(reader: &mut impl Buf) -> Result<u16, Box<dyn Error>> {
        let head = reader.get_u8();
        if reader.remaining() >= 1 && (head & 0x80) == 0 {
            Ok(head as u16)
        } else if reader.remaining() >= 2 && head == Self::INT8_CODE {
            Ok(reader.get_u8() as u16)
        } else if reader.remaining() >= 3 && head == Self::INT16_CODE {
            Ok(reader.get_u16_le())
        } else {
            Err(Box::new(FormatError))
        }
    }

    pub fn get_u32(reader: &mut impl Buf) -> Result<u32, Box<dyn Error>> {
        let head = reader.get_u8();
        if reader.remaining() >= 1 && (head & 0x80) == 0 {
            Ok(head as u32)
        } else if reader.remaining() >= 2 && head == Self::INT8_CODE {
            Ok(reader.get_u8() as u32)
        } else if reader.remaining() >= 3 && head == Self::INT16_CODE {
            Ok(reader.get_u16_le() as u32)
        } else if reader.remaining() >= 5 && head == Self::INT32_CODE {
            Ok(reader.get_u32_le())
        } else {
            Err(Box::new(FormatError))
        }
    }

    pub fn get_u64(reader: &mut impl Buf) -> Result<u64, Box<dyn Error>> {
        let head = reader.get_u8();
        if reader.remaining() >= 1 && (head & 0x80) == 0 {
            Ok(head as u64)
        } else if reader.remaining() >= 2 && head == Self::INT8_CODE {
            Ok(reader.get_u8() as u64)
        } else if reader.remaining() >= 3 && head == Self::INT16_CODE {
            Ok(reader.get_u16_le() as u64)
        } else if reader.remaining() >= 5 && head == Self::INT32_CODE {
            Ok(reader.get_u32_le() as u64)
        } else if reader.remaining() >= 9 && head == Self::INT64_CODE {
            Ok(reader.get_u64_le())
        } else {
            Err(Box::new(FormatError))
        }
    }

    pub fn get_i8(reader: &mut impl Buf) -> Result<i8, Box<dyn Error>> {
        Self::get_u8(reader).map(|value| (value as i8 >> 1) ^ (-(value as i8 & 1)))
    }

    pub fn get_i16(reader: &mut impl Buf) -> Result<i16, Box<dyn Error>> {
        Self::get_u16(reader).map(|value| (value as i16 >> 1) ^ (-(value as i16 & 1)))
    }

    pub fn get_i32(reader: &mut impl Buf) -> Result<i32, Box<dyn Error>> {
        Self::get_u32(reader).map(|value| (value as i32 >> 1) ^ (-(value as i32 & 1)))
    }

    pub fn get_i64(reader: &mut impl Buf) -> Result<i64, Box<dyn Error>> {
        Self::get_u64(reader).map(|value| (value as i64 >> 1) ^ (-(value as i64 & 1)))
    }
}
