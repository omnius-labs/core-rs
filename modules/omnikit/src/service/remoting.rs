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

    impl RocketPackStruct for TestMessage {
        fn pack(encoder: &mut impl RocketPackEncoder, value: &Self) -> std::result::Result<(), RocketPackEncoderError> {
            encoder.write_map(1)?;

            encoder.write_u64(0)?;
            encoder.write_i32(value.value)?;

            Ok(())
        }

        fn unpack(decoder: &mut impl RocketPackDecoder) -> std::result::Result<Self, RocketPackDecoderError>
        where
            Self: Sized,
        {
            let count = decoder.read_map()?;

            let mut value: i32 = 0;

            for _ in 0..count {
                match decoder.read_u64()? {
                    0 => value = decoder.read_i32()?,
                    _ => decoder.skip_field()?,
                }
            }

            Ok(Self { value })
        }
    }
}
