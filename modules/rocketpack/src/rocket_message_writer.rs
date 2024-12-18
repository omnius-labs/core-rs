use tokio_util::bytes::{BufMut, BytesMut};

use crate::{Timestamp64, Timestamp96, Varint};

pub struct RocketMessageWriter<'a> {
    writer: &'a mut BytesMut,
}

impl<'a> RocketMessageWriter<'a> {
    pub fn new(writer: &'a mut BytesMut) -> Self {
        Self { writer }
    }

    pub fn put_str(&mut self, value: &str) {
        Varint::put_u32(value.len() as u32, self.writer);
        self.writer.put_slice(value.as_bytes());
    }

    pub fn put_bytes(&mut self, value: &[u8]) {
        Varint::put_u32(value.len() as u32, self.writer);
        self.writer.put_slice(value);
    }

    pub fn put_timestamp64(&mut self, value: Timestamp64) {
        self.put_i64(value.seconds);
    }

    pub fn put_timestamp96(&mut self, value: Timestamp96) {
        self.put_i64(value.seconds);
        self.put_u32(value.nanos);
    }

    pub fn put_bool(&mut self, value: bool) {
        self.writer.put_u8(if value { 0x01 } else { 0x00 });
    }

    pub fn put_u8(&mut self, value: u8) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn put_u16(&mut self, value: u16) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn put_u32(&mut self, value: u32) {
        Varint::put_u64(value as u64, self.writer);
    }

    pub fn put_u64(&mut self, value: u64) {
        Varint::put_u64(value, self.writer);
    }

    pub fn put_i8(&mut self, value: i8) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn put_i16(&mut self, value: i16) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn put_i32(&mut self, value: i32) {
        Varint::put_i64(value as i64, self.writer);
    }

    pub fn put_i64(&mut self, value: i64) {
        Varint::put_i64(value, self.writer);
    }

    pub fn put_f32(&mut self, value: f32) {
        let bytes = value.to_le_bytes();
        self.writer.put_slice(&bytes);
    }

    pub fn put_f64(&mut self, value: f64) {
        let bytes = value.to_le_bytes();
        self.writer.put_slice(&bytes);
    }
}
