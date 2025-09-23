mod caller;
mod hello_message;
mod listener;
mod stream;

pub use caller::*;
use hello_message::*;
pub use listener::*;
pub use stream::*;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use testresult::TestResult;
    use tokio::io::{AsyncRead, AsyncWrite};

    use crate::prelude::*;

    use super::*;

    #[tokio::test]
    async fn communication_test() -> TestResult {
        const FUNCTION_ID: u32 = 1;

        let (client_side, server_side) = tokio::io::duplex(4096);

        let (client_reader, client_writer) = tokio::io::split(client_side);
        let (server_reader, server_writer) = tokio::io::split(server_side);

        let listener_result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::spawn(async {
                let listener = OmniRemotingListener::<_, _>::new(server_reader, server_writer, 1024 * 1024).await.unwrap();

                async fn callback<R, W>(stream: OmniRemotingStream<R, W>)
                where
                    R: AsyncRead + Send + Unpin + 'static,
                    W: AsyncWrite + Send + Unpin + 'static,
                {
                    let received = stream.recv::<TestMessage>().await.unwrap();
                    info!(value = received.value, "listener receive");

                    stream.send(TestMessage { value: received.value + 1 }).await.unwrap();
                    info!("listener send");
                }

                listener.listen_stream(callback).await.unwrap();

                listener.function_id()
            }),
        );

        let caller_result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::spawn(async {
                let caller = OmniRemotingCaller::<_, _>::new(client_reader, client_writer, 1024 * 1024, FUNCTION_ID).await.unwrap();

                let stream = caller.call_stream();

                stream.send(TestMessage { value: 1 }).await.unwrap();
                info!("caller send");

                let received = stream.recv::<TestMessage>().await.unwrap();
                info!(value = received.value, "caller receive");

                received.value
            }),
        );

        assert_eq!(FUNCTION_ID, listener_result.await??);
        assert_eq!(2, caller_result.await??);

        Ok(())
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TestMessage {
        pub value: i32,
    }

    impl RocketMessage for TestMessage {
        fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
            writer.put_i32(value.value);

            Ok(())
        }

        fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
        where
            Self: Sized,
        {
            let value = reader.get_i32()?;

            Ok(Self { value })
        }
    }
}
