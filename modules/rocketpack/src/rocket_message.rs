use tokio_util::bytes::{Bytes, BytesMut};

use crate::{RocketMessageReader, RocketMessageWriter};

pub trait RocketMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, depth: u32) -> anyhow::Result<()>;

    fn unpack(reader: &mut RocketMessageReader, depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn import(bytes: &mut Bytes) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut reader = RocketMessageReader::new(bytes);
        Self::unpack(&mut reader, 0)
    }

    fn export(&self) -> anyhow::Result<Bytes> {
        let mut bytes = BytesMut::new();
        let mut writer = RocketMessageWriter::new(&mut bytes);
        Self::pack(&mut writer, self, 0)?;
        Ok(bytes.freeze())
    }
}
