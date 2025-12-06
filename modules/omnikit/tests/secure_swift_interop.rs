use std::{path::PathBuf, process::Stdio, sync::Arc, time::Duration};

use parking_lot::Mutex;
use testresult::TestResult;
use tokio::{net::TcpListener, process::Command, time::timeout};
use tokio_util::bytes::Bytes;

use omnius_core_base::{
    clock::ClockUtc,
    random_bytes::{RandomBytesProvider, RandomBytesProviderImpl},
};
use omnius_core_omnikit::service::connection::{
    codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
    secure::{OmniSecureStream, OmniSecureStreamType},
};

const MAX_FRAME_LENGTH: usize = 64 * 1024;
const CHILD_TIMEOUT: Duration = Duration::from_secs(120);

#[ignore]
#[tokio::test]
async fn swift_client_roundtrip() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let mut payload_rng = RandomBytesProviderImpl::new();

    let mut client_payload = vec![0u8; 1024];
    payload_rng.fill_bytes(&mut client_payload);

    let mut server_payload = vec![0u8; 1536];
    payload_rng.fill_bytes(&mut server_payload);

    let swift_package_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../../ui-ios/Libraries/Refs/OmniusCore");

    let child = Command::new("swift")
        .arg("run")
        .arg("--configuration")
        .arg("debug")
        .arg("--package-path")
        .arg(&swift_package_path)
        .arg("InteropClientTest")
        .arg("secure-echo")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--max-frame")
        .arg(MAX_FRAME_LENGTH.to_string())
        .arg("--send-hex")
        .arg(hex::encode(&client_payload))
        .arg("--expect-hex")
        .arg(hex::encode(&server_payload))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (socket, _) = timeout(CHILD_TIMEOUT, listener.accept()).await??;
    let (tcp_reader, tcp_writer) = socket.into_split();

    let random_bytes_provider: Arc<Mutex<dyn RandomBytesProvider + Send + Sync>> = Arc::new(Mutex::new(RandomBytesProviderImpl::new()));
    let clock: Arc<dyn omnius_core_base::clock::Clock<chrono::Utc> + Send + Sync> = Arc::new(ClockUtc);

    let secure_server = OmniSecureStream::new(tcp_reader, tcp_writer, OmniSecureStreamType::Accepted, MAX_FRAME_LENGTH, None, random_bytes_provider, clock).await?;

    let (secure_reader, secure_writer) = tokio::io::split(secure_server);
    let mut receiver = FramedReceiver::new(secure_reader, MAX_FRAME_LENGTH);
    let mut sender = FramedSender::new(secure_writer, MAX_FRAME_LENGTH);

    let received = receiver.recv().await?;
    if received != client_payload {
        return Err("payload from swift did not match".into());
    }

    sender.send(Bytes::from(server_payload)).await?;

    let output = timeout(CHILD_TIMEOUT, child.wait_with_output()).await??;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("swift client failed: status={:?}\nstdout:\n{}\nstderr:\n{}", output.status, stdout, stderr).into());
    }

    Ok(())
}
