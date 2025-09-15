use crate::prelude::*;

pub enum PacketMessage<T>
where
    T: RocketMessage + Send + Sync + 'static,
{
    Unknown,
    Continue(T),
    Completed(T),
}

impl<T> RocketMessage for PacketMessage<T>
where
    T: RocketMessage + Send + Sync + 'static,
{
    fn pack(writer: &mut RocketMessageWriter, value: &Self, depth: u32) -> RocketPackResult<()> {
        if let PacketMessage::Unknown = value {
            writer.put_u8(0);
        } else if let PacketMessage::Continue(value) = value {
            writer.put_u8(1);
            T::pack(writer, value, depth + 1)?;
        } else if let PacketMessage::Completed(value) = value {
            writer.put_u8(2);
            T::pack(writer, value, depth + 1)?;
        }
        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let typ = reader.get_u8()?;

        if typ == 0 {
            Ok(PacketMessage::Unknown)
        } else if typ == 1 {
            let value = T::unpack(reader, depth + 1)?;
            Ok(PacketMessage::Continue(value))
        } else if typ == 2 {
            let value = T::unpack(reader, depth + 1)?;
            Ok(PacketMessage::Completed(value))
        } else {
            Ok(PacketMessage::Unknown)
        }
    }
}
