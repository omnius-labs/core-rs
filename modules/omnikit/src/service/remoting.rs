mod caller;
mod error;
mod error_message;
mod hello_message;
mod listener;
mod packet_message;

pub use caller::*;
pub use error::*;
pub use error_message::*;
use hello_message::*;
pub use listener::*;
use packet_message::*;

#[cfg(test)]
mod tests {
    use omnius_core_rocketpack::{RocketMessage, RocketMessageReader, RocketMessageWriter};
    use testresult::TestResult;
    use tokio::net::TcpListener;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn listen_test() -> TestResult {
        let addr = "0.0.0.0:50000";
        let listener = TcpListener::bind(addr).await?;
        let (reader, writer) = listener.accept().await?.0.into_split();

        let mut listener = OmniRemotingListener::<_, _, OmniRemotingDefaultErrorMessage>::new(reader, writer, 1024 * 1024);
        listener.handshake().await?;

        async fn callback(param: TestMessage) -> Result<TestMessage, OmniRemotingDefaultErrorMessage> {
            Ok(TestMessage { value: param.value + 1 })
        }
        listener.listen(callback).await?;

        Ok(())
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TestMessage {
        pub value: i32,
    }

    impl RocketMessage for TestMessage {
        fn serialize(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) {
            writer.write_i32(value.value);
        }

        fn deserialize(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
        where
            Self: Sized,
        {
            let value = reader.get_i32().map_err(|_| anyhow::anyhow!("invalid typ"))?;

            Ok(Self { value })
        }
    }
}
