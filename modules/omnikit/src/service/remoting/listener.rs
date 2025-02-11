use std::{fmt, future::Future, sync::Arc};

use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex as TokioMutex,
};

use omnius_core_rocketpack::RocketMessage;

use crate::service::connection::codec::{FramedReceiver, FramedRecv as _, FramedSend as _, FramedSender};

use super::{HelloMessage, OmniRemotingVersion, PacketMessage, ProtocolErrorCode};

#[allow(unused)]
pub struct OmniRemotingListener<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
{
    receiver: Arc<TokioMutex<FramedReceiver<R>>>,
    sender: Arc<TokioMutex<FramedSender<W>>>,
    function_id: Arc<Mutex<Option<u32>>>,
    _phantom: std::marker::PhantomData<TError>,
}

impl<R, W, TError> OmniRemotingListener<R, W, TError>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
    TError: RocketMessage + fmt::Display + Send + Sync + 'static,
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

    pub async fn handshake(&mut self) -> Result<(), super::Error<TError>> {
        let mut v = self
            .receiver
            .lock()
            .await
            .recv()
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::ReceiveFailed))?;
        let hello_message = HelloMessage::import(&mut v).map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::DeserializationFailed))?;

        if hello_message.version == OmniRemotingVersion::V1 {
            *self.function_id.lock() = Some(hello_message.function_id);
            return Ok(());
        }

        Err(super::Error::ProtocolError(super::ProtocolErrorCode::UnsupportedVersion))
    }

    pub fn function_id(&self) -> Result<u32, super::Error<TError>> {
        let v = *self.function_id.lock();
        v.ok_or_else(|| super::Error::ProtocolError(super::ProtocolErrorCode::HandshakeNotFinished))
    }

    pub async fn listen<TParam, TResult, F, Fut>(&self, callback: F) -> Result<(), super::Error<TError>>
    where
        TParam: RocketMessage + Send + Sync + 'static,
        TResult: RocketMessage + Send + Sync + 'static,
        F: FnOnce(TParam) -> Fut,
        Fut: Future<Output = Result<TResult, TError>>,
    {
        let mut param = self
            .receiver
            .lock()
            .await
            .recv()
            .await
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::ReceiveFailed))?;
        let param = PacketMessage::<TParam, TError>::import(&mut param)
            .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::DeserializationFailed))?;

        match param {
            PacketMessage::Unknown => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Continue(_) => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
            PacketMessage::Completed(param) => match callback(param).await {
                Ok(result) => {
                    let result = PacketMessage::<TResult, TError>::Completed(result)
                        .export()
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?;
                    self.sender
                        .lock()
                        .await
                        .send(result)
                        .await
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;
                    Ok(())
                }
                Err(error) => {
                    let error = PacketMessage::<TResult, TError>::Error(error)
                        .export()
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SerializationFailed))?;
                    self.sender
                        .lock()
                        .await
                        .send(error)
                        .await
                        .map_err(|_| super::Error::ProtocolError(super::ProtocolErrorCode::SendFailed))?;
                    Ok(())
                }
            },
            PacketMessage::Error(_) => Err(super::Error::ProtocolError(ProtocolErrorCode::UnexpectedProtocol)),
        }
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        self.receiver.lock().await.close().await?;
        self.sender.lock().await.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use omnius_core_rocketpack::{RocketMessageReader, RocketMessageWriter};
    use testresult::TestResult;
    use tokio::{io::AsyncWriteExt, net::TcpListener};
    use tracing::{info, warn};

    use crate::service::remoting::OmniRemotingDefaultErrorMessage;

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
                1 => remoting_listener.listen(callback).await?,
                _ => warn!("not supported"),
            }

            remoting_listener.close().await?;
        }
    }

    pub async fn callback(m: TextMessage) -> Result<TextMessage, OmniRemotingDefaultErrorMessage> {
        Ok(m)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TextMessage {
        pub text: String,
    }

    impl RocketMessage for TextMessage {
        fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> anyhow::Result<()> {
            writer.put_str(&value.text);

            Ok(())
        }

        fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> anyhow::Result<Self>
        where
            Self: Sized,
        {
            let text = reader.get_string(1024)?;
            Ok(Self { text })
        }
    }
}
