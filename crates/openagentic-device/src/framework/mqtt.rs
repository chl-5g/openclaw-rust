//! MQTT 接口
//!
//! MQTT (Message Queuing Telemetry Transport) 协议接口

use crate::framework::FrameworkModule;
use serde::{Deserialize, Serialize};

pub type MqttResult<T> = crate::framework::FrameworkResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Broker not found: {0}")]
    BrokerNotFound(String),
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    #[error("Message too large: {0}")]
    MessageTooLarge(usize),
    #[error("Invalid topic: {0}")]
    InvalidTopic(String),
    #[error("QoS not supported: {0}")]
    QosNotSupported(u8),
    #[error("TLS error: {0}")]
    TlsError(String),
    #[error("Heartbeat error: {0}")]
    HeartbeatError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttHeartbeatConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub topic: String,
    pub qos: MqttQos,
    pub retain: bool,
    pub payload_template: Option<String>,
}

impl Default for MqttHeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 30,
            topic: "device/heartbeat".to_string(),
            qos: MqttQos::AtLeastOnce,
            retain: false,
            payload_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub device_id: String,
    pub timestamp: u64,
    pub status: String,
    pub uptime_secs: u64,
    pub memory_usage_percent: Option<f32>,
    pub cpu_usage_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: MqttQos,
    pub retain: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MqttQos {
    #[default]
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSubscription {
    pub topic: String,
    pub qos: MqttQos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub use_tls: bool,
    pub keep_alive_secs: u16,
    pub clean_session: bool,
    pub last_will: Option<MqttLastWill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttLastWill {
    pub topic: String,
    pub message: Vec<u8>,
    pub qos: MqttQos,
    pub retain: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: "localhost".to_string(),
            port: 1883,
            client_id: None,
            username: None,
            password: None,
            use_tls: false,
            keep_alive_secs: 60,
            clean_session: true,
            last_will: None,
        }
    }
}

pub trait MqttClient: FrameworkModule {
    fn client_id(&self) -> &str;

    fn is_connected(&self) -> bool;

    fn subscribe(&self, topics: &[MqttSubscription]) -> MqttResult<()>;

    fn unsubscribe(&self, topics: &[String]) -> MqttResult<()>;

    fn publish(&self, message: MqttMessage) -> MqttResult<()>;

    fn set_message_callback<F>(&self, callback: F)
    where
        F: Fn(MqttMessage) + Send + Sync + 'static;

    fn pending_messages(&self) -> usize;
}

#[cfg(feature = "mqtt")]
pub mod async_impl {
    use super::*;
    use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS as RumqttQoS};
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};
    use futures_util::{Stream, StreamExt};
    
    

    impl From<MqttQos> for RumqttQoS {
        fn from(qos: MqttQos) -> Self {
            match qos {
                MqttQos::AtMostOnce => RumqttQoS::AtMostOnce,
                MqttQos::AtLeastOnce => RumqttQoS::AtLeastOnce,
                MqttQos::ExactlyOnce => RumqttQoS::ExactlyOnce,
            }
        }
    }

    impl From<RumqttQoS> for MqttQos {
        fn from(qos: RumqttQoS) -> Self {
            match qos {
                RumqttQoS::AtMostOnce => MqttQos::AtMostOnce,
                RumqttQoS::AtLeastOnce => MqttQos::AtLeastOnce,
                RumqttQoS::ExactlyOnce => MqttQos::ExactlyOnce,
            }
        }
    }

    pub struct AsyncMqttClient {
        client: AsyncClient,
        eventloop: Arc<RwLock<EventLoop>>,
        config: MqttConfig,
        message_tx: mpsc::Sender<MqttMessage>,
        message_rx: Arc<RwLock<Option<mpsc::Receiver<MqttMessage>>>>,
        heartbeat_config: MqttHeartbeatConfig,
        heartbeat_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
        start_time: std::time::Instant,
    }

    impl AsyncMqttClient {
        pub async fn new(config: MqttConfig, heartbeat_config: MqttHeartbeatConfig) -> Result<Self, MqttError> {
            let client_id = config
                .client_id
                .clone()
                .unwrap_or_else(|| format!("openagentic-{}", uuid::Uuid::new_v4()));

            let mut mqtt_options = MqttOptions::new(&client_id, &config.broker, config.port);
            mqtt_options.set_keep_alive(std::time::Duration::from_secs(config.keep_alive_secs as u64));
            mqtt_options.set_clean_session(config.clean_session);

            if let (Some(username), Some(password)) = (&config.username, &config.password) {
                mqtt_options.set_credentials(username, password);
            }

            if let Some(ref last_will) = config.last_will {
                let will = rumqttc::LastWill::new(
                    &last_will.topic,
                    last_will.message.clone(),
                    last_will.qos.into(),
                    last_will.retain,
                );
                mqtt_options.set_last_will(will);
            }

            let (client, eventloop) = AsyncClient::new(mqtt_options, 100);
            let (message_tx, message_rx) = mpsc::channel(100);

            Ok(Self {
                client,
                eventloop: Arc::new(RwLock::new(eventloop)),
                config,
                message_tx,
                message_rx: Arc::new(RwLock::new(Some(message_rx))),
                heartbeat_config,
                heartbeat_handle: Arc::new(RwLock::new(None)),
                start_time: std::time::Instant::now(),
            })
        }

        pub async fn connect(&self) -> Result<(), MqttError> {
            let mut eventloop = self.eventloop.write().await;
            loop {
                match eventloop.poll().await {
                    Ok(_) => continue,
                    Err(rumqttc::ConnectionError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                        continue;
                    }
                    Err(e) => return Err(MqttError::ConnectionFailed(e.to_string())),
                }
            }
        }

        pub async fn publish(&self, topic: &str, payload: Vec<u8>, qos: MqttQos, retain: bool) -> Result<(), MqttError> {
            self.client
                .publish(topic, qos.into(), retain, payload)
                .await
                .map_err(|e| MqttError::PublishFailed(e.to_string()))
        }

        pub async fn subscribe(&self, topic: &str, qos: MqttQos) -> Result<(), MqttError> {
            self.client
                .subscribe(topic, qos.into())
                .await
                .map_err(|e| MqttError::SubscribeFailed(e.to_string()))
        }

        pub async fn unsubscribe(&self, topic: &str) -> Result<(), MqttError> {
            self.client
                .unsubscribe(topic)
                .await
                .map_err(|e| MqttError::SubscribeFailed(e.to_string()))
        }

        pub fn get_incoming_messages_stream(&self) -> impl Stream<Item = MqttMessage> + '_ {
            let rx_lock = self.message_rx.clone();
            futures_util::stream::unfold((), move |_| {
                let rx_lock = rx_lock.clone();
                async move {
                    let mut rx_guard = rx_lock.write().await;
                    if let Some(rx) = rx_guard.as_mut() {
                        rx.recv().await.map(|msg| (msg, ()))
                    } else {
                        None
                    }
                }
            })
        }

        pub fn start_heartbeat(&self, device_id: String) {
            if !self.heartbeat_config.enabled {
                return;
            }

            let client = self.client.clone();
            let config = self.heartbeat_config.clone();
            let start_time = self.start_time;
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(config.interval_secs));
                
                loop {
                    interval.tick().await;
                    
                    let payload = HeartbeatPayload {
                        device_id: device_id.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        status: "online".to_string(),
                        uptime_secs: start_time.elapsed().as_secs(),
                        memory_usage_percent: None,
                        cpu_usage_percent: None,
                    };

                    let payload_json = if let Some(ref template) = config.payload_template {
                        template
                            .replace("{device_id}", &payload.device_id)
                            .replace("{timestamp}", &payload.timestamp.to_string())
                            .replace("{status}", &payload.status)
                            .replace("{uptime}", &payload.uptime_secs.to_string())
                    } else {
                        serde_json::to_string(&payload).unwrap_or_default()
                    };

                    let _ = client
                        .publish(&config.topic, config.qos.into(), config.retain, payload_json.into_bytes())
                        .await;
                }
            });

            let handle_guard = self.heartbeat_handle.try_write();
             if let Ok(mut guard) = handle_guard {
                *guard = Some(handle);
            }
        }

        pub fn stop_heartbeat(&self) {
            if let Ok(mut handle_guard) = self.heartbeat_handle.try_write() {
                if let Some(handle) = handle_guard.take() {
                    handle.abort();
                }
            }
        }

        pub fn is_heartbeat_running(&self) -> bool {
            if let Ok(handle_guard) = self.heartbeat_handle.try_read() {
                return handle_guard.is_some();
            }
            false
        }
    }
}
