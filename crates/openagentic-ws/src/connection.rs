use crate::error::{WsError, WsResult};
use crate::message::WsMessageCodec;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct WsConnection<C: WsMessageCodec> {
    pub id: ConnectionId,
    pub user_id: Option<String>,
    sender: mpsc::Sender<C::Message>,
    codec: C,
}

impl<C: WsMessageCodec> WsConnection<C> {
    pub fn new(codec: C) -> Self {
        let (sender, _) = mpsc::channel(100);
        Self {
            id: ConnectionId::new(),
            user_id: None,
            sender,
            codec,
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub async fn send(&self, msg: C::Message) -> WsResult<()> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| WsError::SendFailed(e.to_string()))
    }

    pub fn codec(&self) -> &C {
        &self.codec
    }
}

pub struct WsClient<C: WsMessageCodec> {
    codec: C,
    connection: Arc<RwLock<Option<WsConnection<C>>>>,
    event_tx: broadcast::Sender<C::Message>,
}

impl<C: WsMessageCodec> WsClient<C> {
    pub fn new(codec: C) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            codec,
            connection: Arc::new(RwLock::new(None)),
            event_tx,
        }
    }

    pub async fn connect(&self, url: &str) -> WsResult<WsConnection<C>> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| WsError::ConnectionFailed(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        let (_tx, mut rx) = mpsc::channel::<C::Message>(100);

        let connection = WsConnection::new(self.codec.clone());

        let codec = self.codec.clone();
        let event_tx_clone = self.event_tx.clone();
        let conn_id = connection.id.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        if let Some(m) = msg {
                            let data = codec.encode(&m);
                            let text = String::from_utf8_lossy(&data).to_string();
                            if write.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Some(decoded) = codec.decode(text.as_bytes()) {
                                    let _ = event_tx_clone.send(decoded);
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            tracing::info!("Client connection {} closed", conn_id.0);
        });

        *self.connection.write().await = Some(connection.clone());

        Ok(connection)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<C::Message> {
        self.event_tx.subscribe()
    }

    pub async fn send(&self, msg: C::Message) -> WsResult<()> {
        let conn = self.connection.read().await;
        if let Some(ref c) = *conn {
            c.send(msg).await
        } else {
            Err(WsError::NotConnected)
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.connection.read().await.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::JsonCodec;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestMsg {
        content: String,
    }

    #[test]
    fn test_connection_id() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_json_codec_in_connection() {
        let codec = JsonCodec::<TestMsg>::new();
        let conn = WsConnection::new(codec);
        assert!(conn.user_id.is_none());
    }

    #[test]
    fn test_connection_with_user_id() {
        let codec = JsonCodec::<TestMsg>::new();
        let conn = WsConnection::new(codec).with_user_id("user-123".to_string());
        assert_eq!(conn.user_id, Some("user-123".to_string()));
    }
}
