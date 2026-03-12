use crate::connection::WsConnection;
use crate::message::WsMessageCodec;
use crate::room::WsRoom;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WsServerState<C: WsMessageCodec> {
    pub room: Arc<WsRoom<C>>,
}

pub fn create_websocket_route<C: WsMessageCodec>(
    path: &str,
    room: Arc<WsRoom<C>>,
) -> Router {
    let state = WsServerState { room };

    Router::new()
        .route(path, get(websocket_handler))
        .with_state(Arc::new(RwLock::new(state)))
}

async fn websocket_handler<C: WsMessageCodec>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<WsServerState<C>>>>,
) -> Response {
    let room = {
        let s = state.read().await;
        s.room.clone()
    };

    ws.on_upgrade(move |socket| handle_socket(socket, room))
}

async fn handle_socket<C: WsMessageCodec>(socket: WebSocket, room: Arc<WsRoom<C>>) {
    let (mut _sender, mut receiver) = socket.split();

    let connection = WsConnection::new(room.codec().clone());
    let conn_id = connection.id.clone();

    room.join(connection.clone()).await.ok();

    let codec = room.codec().clone();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(decoded) = codec.decode(text.as_bytes()) {
                            let _ = room.broadcast(&decoded).await;
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

    room.leave(&conn_id).await.ok();
    tracing::info!("WebSocket connection {} closed", conn_id.0);
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
    fn test_server_state() {
        let codec = JsonCodec::<TestMsg>::new();
        let room = Arc::new(WsRoom::new(
            crate::room::RoomId::new("test"),
            codec,
        ));
        let state = WsServerState { room };
        assert_eq!(state.room.id().0, "test");
    }
}
