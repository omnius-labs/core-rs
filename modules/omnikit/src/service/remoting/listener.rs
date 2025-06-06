use std::sync::Arc;

use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::RocketMessage;

use crate::{
    prelude::*,
    service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender},
};

use super::{HelloMessage, OmniRemotingStream, OmniRemotingVersion, PacketMessage};

#[allow(unused)]
pub struct OmniRemotingListener<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: Arc<Mutex<Option<u32>>>,
    _phantom: std::marker::PhantomData<TErrorMessage>,
}

impl<R, W, TErrorMessage> OmniRemotingListener<R, W, TErrorMessage>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TErrorMessage: RocketMessage + std::fmt::Display + Send + Sync + 'static,
{
    pub fn new(reader: R, writer: W, max_frame_length: usize) -> Self {
        let receiver = Arc::new(TokioMutex::new(FramedReceiver::new(reader, max_frame_length)));
        let sender = Arc::new(TokioMutex::new(FramedSender::new(writer, max_frame_length)));

        OmniRemotingListener {
            sender,
            receiver,
            function_id: Arc::new(Mutex::new(None)),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn handshake(&mut self) -> Result<()> {
        let mut v = self.receiver.lock().await.recv().await?;
        let hello_message = HelloMessage::import(&mut v)?;

        if hello_message.version == OmniRemotingVersion::V1 {
            *self.function_id.lock() = Some(hello_message.function_id);
            return Ok(());
        }

        Err(Error::new(ErrorKind::UnsupportedVersion).message(format!("unsupported version: {}", hello_message.version)))
    }

    pub fn function_id(&self) -> Result<u32> {
        let v = *self.function_id.lock();
        Ok(v.ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotConnected))?)
    }

    pub async fn listen_unary<TParamMessage, TSuccessMessage, F>(&self, callback: F) -> Result<()>
    where
        TParamMessage: RocketMessage + Send + Sync + 'static,
        TSuccessMessage: RocketMessage + Send + Sync + 'static,
        F: AsyncFnOnce(TParamMessage) -> std::result::Result<TSuccessMessage, TErrorMessage>,
    {
        let mut param = self.receiver.lock().await.recv().await?;
        let param = PacketMessage::<TParamMessage, TErrorMessage>::import(&mut param)?;

        match param {
            PacketMessage::Unknown => Err(Error::new(ErrorKind::UnsupportedType).message("type unknown")),
            PacketMessage::Continue(_) => Err(Error::new(ErrorKind::UnsupportedType).message("type continue")),
            PacketMessage::Completed(param) => match callback(param).await {
                Ok(message) => {
                    let message = PacketMessage::<TSuccessMessage, TErrorMessage>::Completed(message).export()?;
                    self.sender.lock().await.send(message).await?;
                    Ok(())
                }
                Err(error_message) => {
                    let error_message = PacketMessage::<TSuccessMessage, TErrorMessage>::Error(error_message).export()?;
                    self.sender.lock().await.send(error_message).await?;
                    Ok(())
                }
            },
            PacketMessage::Error(_) => Err(Error::new(ErrorKind::UnsupportedType).message("type error")),
        }
    }

    pub async fn listen_stream<F>(&self, callback: F) -> Result<()>
    where
        F: AsyncFnOnce(OmniRemotingStream<R, W, TErrorMessage>),
    {
        callback(OmniRemotingStream::new(self.receiver.clone(), self.sender.clone())).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;
    use tokio::net::TcpListener;
    use tracing::{info, warn};

    use crate::{prelude::*, service::remoting::OmniRemotingDefaultErrorMessage};

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn multi_language_communication_listener_test() -> TestResult {
        let tcp_listener = TcpListener::bind("0.0.0.0:50000").await?;

        loop {
            let (tcp_stream, _) = tcp_listener.accept().await?;
            let (reader, writer) = tokio::io::split(tcp_stream);

            info!("listen start");

            let mut remoting_listener = OmniRemotingListener::<_, _, OmniRemotingDefaultErrorMessage>::new(reader, writer, 1024 * 1024);
            remoting_listener.handshake().await?;

            match remoting_listener.function_id()? {
                1 => remoting_listener.listen_unary(callback).await?,
                _ => warn!("not supported"),
            }
        }
    }

    pub async fn callback(m: TextMessage) -> std::result::Result<TextMessage, OmniRemotingDefaultErrorMessage> {
        Ok(m)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TextMessage {
        pub text: String,
    }

    impl RocketMessage for TextMessage {
        fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
            writer.put_str(&value.text);

            Ok(())
        }

        fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
        where
            Self: Sized,
        {
            let text = reader.get_string(1024)?;
            Ok(Self { text })
        }
    }
}
