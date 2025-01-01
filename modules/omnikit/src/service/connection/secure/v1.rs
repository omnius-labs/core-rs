mod auth;
mod decoder;
mod encoder;
mod message;
mod stream;
mod util;

use auth::*;
use decoder::*;
use encoder::*;
use message::*;
pub use stream::*;

#[cfg(test)]
mod tests {
    use core::str;
    use std::sync::Arc;

    use chrono::DateTime;
    use futures_util::SinkExt as _;
    use parking_lot::Mutex;
    use testresult::TestResult;
    use tokio::{
        net::{TcpListener, TcpStream},
        time::sleep,
    };
    use tokio_stream::StreamExt as _;

    use omnius_core_base::{
        clock::FakeClockUtc,
        random_bytes::{RandomBytesProvider, RandomBytesProviderImpl},
    };
    use tokio_util::bytes::Bytes;

    use crate::service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender};

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn simple_test() -> TestResult {
        let clock = Arc::new(FakeClockUtc::new(DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap().into()));
        let random_bytes_provider = Arc::new(Mutex::new(RandomBytesProviderImpl::new()));

        let addr = "127.0.0.1:50001";
        let listener = TcpListener::bind(addr).await?;
        let (client_reader, client_writer) = TcpStream::connect(addr).await?.into_split();
        let (server_reader, server_writer) = listener.accept().await?.0.into_split();

        let secure_client = OmniSecureStream::new(
            client_reader,
            client_writer,
            OmniSecureStreamType::Connected,
            1024,
            None,
            random_bytes_provider.clone(),
            clock.clone(),
        );
        let secure_server = OmniSecureStream::new(
            server_reader,
            server_writer,
            OmniSecureStreamType::Accepted,
            1024,
            None,
            random_bytes_provider.clone(),
            clock.clone(),
        );

        let (secure_client, secure_server) = tokio::try_join!(secure_client, secure_server)?;

        let mut secure_client_sender = FramedSender::new(secure_client, 1024 * 1024 * 32);
        let mut secure_server_receiver = FramedReceiver::new(secure_server, 1024 * 1024 * 32);

        let cases = [1, 2, 3, 10, 100, 1000, 1024 * 1024];
        for &case in cases.iter() {
            let mut buffer = vec![0u8; case];
            random_bytes_provider.clone().lock().fill_bytes(&mut buffer);
            let buffer = Bytes::from(buffer);

            secure_client_sender.send(buffer.clone()).await?;
            let received = secure_server_receiver.recv().await?;
            assert_eq!(buffer, received);
        }

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn server_echo_test() -> TestResult {
        loop {
            let clock = Arc::new(FakeClockUtc::new(DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap().into()));
            let random_bytes_provider = Arc::new(Mutex::new(RandomBytesProviderImpl::new()));

            let addr = "0.0.0.0:50000";
            let listener = TcpListener::bind(addr).await?;
            let (server_reader, server_writer) = listener.accept().await?.0.into_split();

            let secure_server = OmniSecureStream::new(
                server_reader,
                server_writer,
                OmniSecureStreamType::Accepted,
                1024,
                None,
                random_bytes_provider.clone(),
                clock.clone(),
            )
            .await?;

            let codec = tokio_util::codec::LengthDelimitedCodec::builder()
                .max_frame_length(1024)
                .little_endian()
                .new_codec();
            let mut framed = tokio_util::codec::Framed::new(secure_server, codec);

            let buffer = framed.next().await.ok_or_else(|| anyhow::anyhow!("Stream ended"))??;

            let s = str::from_utf8(buffer.as_ref())?.to_string();
            println!("{}", s);

            framed.send(buffer.freeze()).await?;
            framed.flush().await?;

            sleep(std::time::Duration::from_millis(3000)).await;
        }
    }
}
