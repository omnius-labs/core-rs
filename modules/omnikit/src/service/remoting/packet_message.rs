use omnius_core_rocketpack::{RocketMessage, RocketMessageReader, RocketMessageWriter};

pub enum PacketMessage<T, E>
where
    T: RocketMessage + Send + Sync + 'static,
    E: RocketMessage + Send + Sync + 'static,
{
    Unknown,
    Continue(T),
    Completed(T),
    Error(E),
}

impl<T, E> RocketMessage for PacketMessage<T, E>
where
    T: RocketMessage + Send + Sync + 'static,
    E: RocketMessage + Send + Sync + 'static,
{
    fn serialize(writer: &mut RocketMessageWriter, value: &Self, depth: u32) {
        if let PacketMessage::Unknown = value {
            writer.write_u8(0);
        } else if let PacketMessage::Continue(value) = value {
            writer.write_u8(1);
            T::serialize(writer, value, depth + 1);
        } else if let PacketMessage::Completed(value) = value {
            writer.write_u8(2);
            T::serialize(writer, value, depth + 1);
        } else if let PacketMessage::Error(value) = value {
            writer.write_u8(3);
            E::serialize(writer, value, depth + 1);
        }
    }

    fn deserialize(reader: &mut RocketMessageReader, depth: u32) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let typ = reader.get_u8().map_err(|_| anyhow::anyhow!("invalid type"))?;

        if typ == 0 {
            Ok(PacketMessage::Unknown)
        } else if typ == 1 {
            let value = T::deserialize(reader, depth + 1)?;
            Ok(PacketMessage::Continue(value))
        } else if typ == 2 {
            let value = T::deserialize(reader, depth + 1)?;
            Ok(PacketMessage::Completed(value))
        } else if typ == 3 {
            let value = E::deserialize(reader, depth + 1)?;
            Ok(PacketMessage::Error(value))
        } else {
            Err(anyhow::anyhow!("invalid type"))
        }
    }
}
