mod caller;
mod listener;
mod stream;

use crate::generated::omnius::core::omnikit::{HelloMessage, OmniRemotingVersion};

pub use caller::*;
pub use listener::*;
pub use stream::*;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use testresult::TestResult;

    use crate::generated::omnius::core::omnikit::TestMessage;
    use crate::prelude::*;

    use super::*;

    #[tokio::test]
    async fn communication_test() -> TestResult {
        const FUNCTION_ID: u32 = 1;

        let (client_side, server_side) = tokio::io::duplex(4096);

        let listener_result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::spawn(async {
                let listener = OmniRemotingListener::new(server_side, 1024 * 1024).await.unwrap();

                async fn callback(stream: OmniRemotingStream) {
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
                let caller = OmniRemotingCaller::new(client_side, 1024 * 1024, FUNCTION_ID).await.unwrap();

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
}
