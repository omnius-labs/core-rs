mod auth;
mod decoder;
mod encoder;
mod message;
mod packet;
mod stream;
mod util;

#[allow(unused)]
use auth::*;
use decoder::*;
use encoder::*;
use message::*;
use packet::*;
pub use stream::*;
#[allow(unused)]
use util::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::DateTime;
    use testresult::TestResult;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use omnius_core_base::clock::FakeClockUtc;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn simple_test() -> TestResult {
        let clock = Arc::new(FakeClockUtc::new(DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z").unwrap().into()));

        let addr = "127.0.0.1:50000";
        let listener = TcpListener::bind(addr).await?;
        let (client_reader, client_writer) = TcpStream::connect(addr).await?.into_split();
        let (server_reader, server_writer) = listener.accept().await?.0.into_split();

        let secure_client = SecureStream::new(client_reader, client_writer, OmniSecureStreamType::Connected, 1024, None, clock.clone());
        let secure_server = SecureStream::new(server_reader, server_writer, OmniSecureStreamType::Accepted, 1024, None, clock.clone());

        let (mut secure_client, mut secure_server) = tokio::try_join!(secure_client, secure_server)?;

        secure_client.write_i32_le(10).await?;
        secure_client.flush().await?;
        assert_eq!(10, secure_server.read_i32_le().await?);

        Ok(())
    }
}
