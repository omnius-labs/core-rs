use p256::pkcs8::der::Writer;
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::bytes::{Buf, BufMut, Bytes, BytesMut};

enum PacketType {
    #[allow(unused)]
    Unknown = 0,
    Cbor = 1,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            1 => PacketType::Cbor,
            _ => PacketType::Unknown,
        }
    }
}

pub struct Packet;

impl Packet {
    pub fn serialize<T: Serialize>(item: T) -> anyhow::Result<Bytes> {
        let buffer = BytesMut::new();
        let mut writer = buffer.writer();

        writer.write_byte(PacketType::Cbor as u8)?;

        ciborium::ser::into_writer(&item, &mut writer)?;
        let buffer = writer.into_inner().freeze();
        Ok(buffer)
    }

    pub fn deserialize<T: DeserializeOwned>(buf: Bytes) -> anyhow::Result<T> {
        let mut buf = buf;

        if buf.is_empty() {
            return Err(anyhow::anyhow!("Invalid packet"));
        }

        match PacketType::from(buf.get_u8()) {
            PacketType::Cbor => {
                let mut reader = buf.reader();
                let item = ciborium::de::from_reader(&mut reader)?;
                Ok(item)
            }
            _ => Err(anyhow::anyhow!("Invalid packet type")),
        }
    }
}
