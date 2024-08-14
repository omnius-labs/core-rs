use tokio_util::bytes::{Bytes, BytesMut};

use crate::{RocketMessageReader, RocketMessageWriter};

pub trait RocketMessage {
    fn serialize(writer: &mut RocketMessageWriter, value: &Self, depth: u32);

    fn deserialize(reader: &mut RocketMessageReader, depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn import(bytes: &mut Bytes) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut reader = RocketMessageReader::new(bytes);
        Self::deserialize(&mut reader, 0)
    }

    fn export(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        let mut writer = RocketMessageWriter::new(&mut bytes);
        Self::serialize(&mut writer, self, 0);
        bytes.freeze()
    }
}
