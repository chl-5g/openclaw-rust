use crate::connection::{ConnectionId, WsConnection};
use crate::error::{WsError, WsResult};
use crate::message::WsMessageCodec;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RoomId(pub String);

impl RoomId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new(Uuid::new_v4().to_string())
    }
}

pub struct WsRoom<C: WsMessageCodec> {
    id: RoomId,
    connections: Arc<RwLock<HashMap<ConnectionId, WsConnection<C>>>>,
    event_tx: broadcast::Sender<RoomEvent<C::Message>>,
    codec: C,
}

impl<C: WsMessageCodec> WsRoom<C> {
    pub fn new(id: RoomId, codec: C) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            id,
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            codec,
        }
    }

    pub fn id(&self) -> &RoomId {
        &self.id
    }

    pub fn codec(&self) -> &C {
        &self.codec
    }

    pub async fn join(&self, connection: WsConnection<C>) -> WsResult<()> {
        let conn_id = connection.id.clone();
        let user_id = connection.user_id.clone();

        {
            let mut connections = self.connections.write().await;
            connections.insert(conn_id.clone(), connection);
        }

        let event = RoomEvent::UserJoined {
            room_id: self.id.0.clone(),
            connection_id: conn_id.clone(),
            user_id,
        };
        let _ = self.event_tx.send(event);

        tracing::info!("Connection {} joined room {}", conn_id.0, self.id.0);
        Ok(())
    }

    pub async fn leave(&self, conn_id: &ConnectionId) -> WsResult<()> {
        let user_id = {
            let mut connections = self.connections.write().await;
            let user_id = connections.get(conn_id).and_then(|c| c.user_id.clone());
            if connections.remove(conn_id).is_none() {
                return Err(WsError::ConnectionNotFound(conn_id.0.clone()));
            }
            user_id
        };

        let event = RoomEvent::UserLeft {
            room_id: self.id.0.clone(),
            connection_id: conn_id.clone(),
            user_id,
        };
        let _ = self.event_tx.send(event);

        tracing::info!("Connection {} left room {}", conn_id.0, self.id.0);
        Ok(())
    }

    pub async fn broadcast(&self, msg: &C::Message) -> WsResult<()> {
        let connections = self.connections.read().await;
        let mut errors = Vec::new();

        for (conn_id, connection) in connections.iter() {
            if let Err(e) = connection.send(msg.clone()).await {
                errors.push(format!("{}: {}", conn_id.0, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(WsError::SendFailed(errors.join("; ")))
        }
    }

    pub async fn send_to(&self, conn_id: &ConnectionId, msg: &C::Message) -> WsResult<()> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(conn_id) {
            connection.send(msg.clone()).await
        } else {
            Err(WsError::ConnectionNotFound(conn_id.0.clone()))
        }
    }

    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    pub async fn get_connections(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RoomEvent<C::Message>> {
        self.event_tx.subscribe()
    }
}

#[derive(Clone, Debug)]
pub enum RoomEvent<M> {
    UserJoined {
        room_id: String,
        connection_id: ConnectionId,
        user_id: Option<String>,
    },
    UserLeft {
        room_id: String,
        connection_id: ConnectionId,
        user_id: Option<String>,
    },
    Message {
        room_id: String,
        connection_id: ConnectionId,
        message: M,
    },
}

pub struct WsRoomManager<C: WsMessageCodec> {
    rooms: Arc<RwLock<HashMap<String, Arc<WsRoom<C>>>>>,
    codec: C,
}

impl<C: WsMessageCodec> WsRoomManager<C> {
    pub fn new(codec: C) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            codec,
        }
    }

    pub async fn create_room(&self, room_id: impl Into<String>) -> WsResult<Arc<WsRoom<C>>> {
        let id = room_id.into();
        let mut rooms = self.rooms.write().await;

        if rooms.contains_key(&id) {
            return Err(WsError::RoomAlreadyExists(id));
        }

        let room = Arc::new(WsRoom::new(RoomId::new(&id), self.codec.clone()));
        rooms.insert(id.clone(), room.clone());

        tracing::info!("Created room: {}", id);
        Ok(room)
    }

    pub async fn get_room(&self, room_id: &str) -> WsResult<Arc<WsRoom<C>>> {
        let rooms = self.rooms.read().await;
        rooms
            .get(room_id)
            .cloned()
            .ok_or_else(|| WsError::RoomNotFound(room_id.to_string()))
    }

    pub async fn get_or_create_room(&self, room_id: impl Into<String>) -> Arc<WsRoom<C>> {
        let id = room_id.into();
        {
            let rooms = self.rooms.read().await;
            if let Some(room) = rooms.get(&id) {
                return room.clone();
            }
        }
        self.create_room(id).await.unwrap()
    }

    pub async fn remove_room(&self, room_id: &str) -> WsResult<()> {
        let mut rooms = self.rooms.write().await;
        if rooms.remove(room_id).is_none() {
            return Err(WsError::RoomNotFound(room_id.to_string()));
        }
        tracing::info!("Removed room: {}", room_id);
        Ok(())
    }

    pub async fn list_rooms(&self) -> Vec<String> {
        let rooms = self.rooms.read().await;
        rooms.keys().cloned().collect()
    }

    pub async fn room_count(&self) -> usize {
        self.rooms.read().await.len()
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

    #[tokio::test]
    async fn test_room_manager_create_room() {
        let codec = JsonCodec::<TestMsg>::new();
        let manager = WsRoomManager::new(codec);

        let room = manager.create_room("test-room").await;
        assert!(room.is_ok());
    }

    #[tokio::test]
    async fn test_room_manager_duplicate_room() {
        let codec = JsonCodec::<TestMsg>::new();
        let manager = WsRoomManager::new(codec);

        manager.create_room("test-room").await.unwrap();
        let result = manager.create_room("test-room").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_room_join_leave() {
        let codec = JsonCodec::<TestMsg>::new();
        let room = WsRoom::new(RoomId::new("test"), codec);

        let conn = WsConnection::new(JsonCodec::<TestMsg>::new())
            .with_user_id("user-1".to_string());

        let conn_id = conn.id.clone();
        room.join(conn).await.unwrap();

        assert_eq!(room.connection_count().await, 1);

        room.leave(&conn_id).await.unwrap();
        assert_eq!(room.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_room_broadcast() {
        let codec = JsonCodec::<TestMsg>::new();
        let room = WsRoom::new(RoomId::new("test"), codec.clone());

        let conn1 = WsConnection::new(codec.clone())
            .with_user_id("user-1".to_string());
        let conn2 = WsConnection::new(codec)
            .with_user_id("user-2".to_string());

        room.join(conn1).await.unwrap();
        room.join(conn2).await.unwrap();

        let msg = TestMsg {
            content: "Hello".to_string(),
        };
        let _ = room.broadcast(&msg).await;
        assert_eq!(room.connection_count().await, 2);
    }
}
