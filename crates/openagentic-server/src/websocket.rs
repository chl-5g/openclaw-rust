//! WebSocket 支持

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::Response,
    routing::get,
    Router,
};
use futures::StreamExt;
use openagentic_ws::{JsonCodec, RoomId, WsRoom, WsServerState, WsMessageCodec};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    pub msg_type: String,
    pub content: String,
}

pub struct AppWebSocketState {
    pub room: Arc<WsRoom<JsonCodec<ServerMessage>>>,
}

impl AppWebSocketState {
    pub fn new() -> Self {
        let codec = JsonCodec::<ServerMessage>::new();
        let room = Arc::new(WsRoom::new(RoomId::new("app"), codec));
        Self { room }
    }
}

impl Default for AppWebSocketState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn websocket_router() -> Router {
    let state = AppWebSocketState::new();
    let room = state.room.clone();

    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(Arc::new(RwLock::new(WsServerState { room })))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<WsServerState<JsonCodec<ServerMessage>>>>>,
) -> Response {
    let room = {
        let s = state.read().await;
        s.room.clone()
    };

    ws.on_upgrade(move |socket| handle_socket(socket, room))
}

async fn handle_socket(
    socket: WebSocket,
    room: Arc<WsRoom<JsonCodec<ServerMessage>>>,
) {
    use openagentic_ws::WsConnection;

    let (mut _sender, mut receiver) = socket.split();

    let codec = room.codec().clone();
    let connection = WsConnection::new(codec.clone());
    let conn_id = connection.id.clone();

    room.join(connection).await.ok();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(decoded) = codec.decode(text.as_bytes()) {
                            tracing::info!("Received from {}: {:?}", conn_id.0, decoded);
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
