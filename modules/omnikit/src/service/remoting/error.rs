use std::fmt;

use omnius_core_rocketpack::RocketMessage;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error<TErrorMessage>
where
    TErrorMessage: RocketMessage + fmt::Display + Send + Sync + 'static,
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
    SerializationFailed,
    DeserializationFailed,
    HandshakeNotFinished,
}

impl fmt::Display for ProtocolErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolErrorCode::UnexpectedProtocol => write!(f, "UnexpectedProtocol"),
            ProtocolErrorCode::UnsupportedVersion => write!(f, "UnsupportedVersion"),
            ProtocolErrorCode::SendFailed => write!(f, "SendFailed"),
            ProtocolErrorCode::ReceiveFailed => write!(f, "ReceiveFailed"),
            ProtocolErrorCode::SerializationFailed => write!(f, "SerializationFailed"),
            ProtocolErrorCode::DeserializationFailed => write!(f, "DeserializationFailed"),
            ProtocolErrorCode::HandshakeNotFinished => write!(f, "HandshakeNotFinished"),
        }
    }
}

impl<TErrorMessage> fmt::Display for Error<TErrorMessage>
where
    TErrorMessage: RocketMessage + fmt::Display + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ApplicationError(error_message) => {
                write!(f, "ApplicationError: {}", error_message)
            }
            Error::ProtocolError(error_code) => write!(f, "ProtocolError: {}", error_code),
        }
    }
}
