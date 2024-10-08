use core::fmt;

use omnius_core_rocketpack::RocketMessage;

#[derive(Debug, Clone)]
pub enum Error<TErrorMessage>
where
    TErrorMessage: RocketMessage + Send + Sync + 'static,
{
    ApplicationError(TErrorMessage),
    ProtocolError(ProtocolErrorCode),
}

#[derive(Debug, Clone)]
pub enum ProtocolErrorCode {
    UnexpectedProtocol,
    UnsupportedVersion,
    SendFailed,
    ReceiveFailed,
    SerializeFailed,
    DeserializeFailed,
}

impl<TErrorMessage> fmt::Display for Error<TErrorMessage>
where
    TErrorMessage: RocketMessage + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ApplicationError(_) => write!(f, "ApplicationError"),
            Error::ProtocolError(_) => write!(f, "ProtocolError"),
        }
    }
}
