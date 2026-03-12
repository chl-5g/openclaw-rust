use thiserror::Error;

#[derive(Debug, Error)]
pub enum WsError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Room already exists: {0}")]
    RoomAlreadyExists(String),

    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("Bind failed: {0}")]
    BindFailed(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Channel closed")]
    ChannelClosed,
}

pub type WsResult<T> = Result<T, WsError>;
