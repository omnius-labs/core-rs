use tokio_util::bytes::{BufMut, BytesMut};

use crate::{Timestamp64, Timestamp96, Varint};

pub struct RocketMessageWriter<'a> {
    writer: &'a mut BytesMut,
}

impl<'a> RocketMessageWriter<'a> {
    pub fn new(writer: &'a mut BytesMut) -> Self {
        Self { writer }
    }

    pub fn write_str(&mut self, value: &str) {
        Varint::put_u32(value.len() as u32, self.writer);
        self.writer.put_slice(value.as_bytes());
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        Varint::put_u32(value.len() as u32, self.writer);
        self.writer.put_slice(value);
    }

    pub fn write_timestamp64(&mut self, value: Timestamp64) {
        self.write_i64(value.seconds);
    }

    pub fn write_timestamp96(&mut self, value: Timestamp96) {
        self.write_i64(value.seconds);
        self.write_u32(value.nanos);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.writer.put_u8(if value { 0x01 } else { 0x00 });
    }

    pub fn write_u8(&mut self, value: u8) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn write_u16(&mut self, value: u16) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn write_u32(&mut self, value: u32) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn write_u64(&mut self, value: u64) {
        Varint::put_u64(value, self.writer);
    }

    pub fn write_i8(&mut self, value: i8) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn write_i16(&mut self, value: i16) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn write_i32(&mut self, value: i32) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn write_i64(&mut self, value: i64) {
        Varint::put_i64(value, self.writer);
    }

    pub fn write_f32(&mut self, value: f32) {
        let bytes = value.to_le_bytes();
        self.writer.put_slice(&bytes);
    }

    pub fn write_f64(&mut self, value: f64) {
        let bytes = value.to_le_bytes();
        self.writer.put_slice(&bytes);
    }
}
