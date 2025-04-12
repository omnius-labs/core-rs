use tokio_util::bytes::{Bytes, BytesMut};

use crate::{RocketMessageReader, RocketMessageWriter, prelude::*};

pub trait RocketMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, depth: u32) -> Result<()>;

    fn unpack(reader: &mut RocketMessageReader, depth: u32) -> Result<Self>
    where
        Self: Sized;

    fn import(bytes: &mut Bytes) -> Result<Self>
    where
        Self: Sized,
    {
        let mut reader = RocketMessageReader::new(bytes);
        Self::unpack(&mut reader, 0)
    }

    fn export(&self) -> Result<Bytes> {
        let mut bytes = BytesMut::new();
        let mut writer = RocketMessageWriter::new(&mut bytes);
        Self::pack(&mut writer, self, 0)?;
        Ok(bytes.freeze())
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn simple_test() -> TestResult {
        Ok(())
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TestMessage {
        pub value: i32,
    }

    impl RocketMessage for TestMessage {
        fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> Result<()> {
            writer.put_i32(value.value);

            Ok(())
        }

        fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> Result<Self>
        where
            Self: Sized,
        {
            let value = reader.get_i32()?;

            Ok(Self { value })
        }
    }
}
