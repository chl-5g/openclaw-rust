use std::collections::HashMap;
use std::time::Instant;

use crate::heartbeat::{HeartbeatData, HeartbeatProvider, DeviceStatus, DeviceMetrics, current_timestamp};

pub trait RobotHeartbeatProvider: HeartbeatProvider {
    fn get_node_names(&self) -> Vec<String>;
    fn get_active_topics(&self) -> Vec<String>;
    fn get_cpu_temperature(&self) -> Option<f32>;
}

#[derive(Debug, Clone, Default)]
pub struct RobotMetrics {
    pub node_names: Vec<String>,
    pub active_topics: Vec<String>,
    pub cpu_temperature: Option<f32>,
}

pub struct JetsonHeartbeatProvider {
    id: String,
    model: String,
    gpu_usage: f32,
    gpu_memory_used: u64,
    gpu_memory_total: u64,
    gpu_temperature: f32,
    power_draw: f32,
    cpu_usage: f32,
    memory_usage: f32,
    cpu_temperature: f32,
    start_time: Instant,
    status: DeviceStatus,
}

impl JetsonHeartbeatProvider {
    pub fn new(id: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            model: model.to_string(),
            gpu_usage: 0.0,
            gpu_memory_used: 0,
            gpu_memory_total: 8 * 1024 * 1024 * 1024,
            gpu_temperature: 40.0,
            power_draw: 5.0,
            cpu_usage: 10.0,
            memory_usage: 30.0,
            cpu_temperature: 45.0,
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_gpu_metrics(mut self, usage: f32, mem_used: u64, temp: f32, power: f32) -> Self {
        self.gpu_usage = usage;
        self.gpu_memory_used = mem_used;
        self.gpu_temperature = temp;
        self.power_draw = power;
        self
    }

    pub fn with_cpu_metrics(mut self, usage: f32, mem_usage: f32, temp: f32) -> Self {
        self.cpu_usage = usage;
        self.memory_usage = mem_usage;
        self.cpu_temperature = temp;
        self
    }
}

impl RobotHeartbeatProvider for JetsonHeartbeatProvider {
    fn get_node_names(&self) -> Vec<String> {
        vec![]
    }

    fn get_active_topics(&self) -> Vec<String> {
        vec![]
    }

    fn get_cpu_temperature(&self) -> Option<f32> {
        Some(self.cpu_temperature)
    }
}

impl HeartbeatProvider for JetsonHeartbeatProvider {
    fn provider_name(&self) -> &str {
        "jetson"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("model".to_string(), self.model.clone());
        custom_fields.insert("gpu_usage_percent".to_string(), format!("{:.1}", self.gpu_usage));
        custom_fields.insert("gpu_memory_used".to_string(), format!("{} MB", self.gpu_memory_used / (1024 * 1024)));
        custom_fields.insert("gpu_memory_total".to_string(), format!("{} MB", self.gpu_memory_total / (1024 * 1024)));
        custom_fields.insert("gpu_temperature".to_string(), format!("{:.1}°C", self.gpu_temperature));
        custom_fields.insert("power_draw".to_string(), format!("{:.2} W", self.power_draw));

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "jetson".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: Some(self.cpu_usage),
                memory_usage_percent: Some(self.memory_usage),
                temperature_celsius: Some(self.gpu_temperature),
                battery_percent: None,
                network_connected: true,
            },
            custom_fields,
        }
    }

    fn get_device_status(&self) -> DeviceStatus {
        self.status
    }
}

pub struct Ros2HeartbeatProvider {
    id: String,
    node_name: String,
    namespace: String,
    ros_distro: String,
    domain_id: u32,
    active_topics: Vec<String>,
    active_nodes: Vec<String>,
    start_time: Instant,
    status: DeviceStatus,
}

impl Ros2HeartbeatProvider {
    pub fn new(id: &str, node_name: &str) -> Self {
        Self {
            id: id.to_string(),
            node_name: node_name.to_string(),
            namespace: "/".to_string(),
            ros_distro: "jazzy".to_string(),
            domain_id: 0,
            active_topics: vec![],
            active_nodes: vec![],
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_ros_info(mut self, distro: &str, domain: u32) -> Self {
        self.ros_distro = distro.to_string();
        self.domain_id = domain;
        self
    }
}

impl RobotHeartbeatProvider for Ros2HeartbeatProvider {
    fn get_node_names(&self) -> Vec<String> {
        self.active_nodes.clone()
    }

    fn get_active_topics(&self) -> Vec<String> {
        self.active_topics.clone()
    }

    fn get_cpu_temperature(&self) -> Option<f32> {
        None
    }
}

impl HeartbeatProvider for Ros2HeartbeatProvider {
    fn provider_name(&self) -> &str {
        "ros2"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("node_name".to_string(), self.node_name.clone());
        custom_fields.insert("namespace".to_string(), self.namespace.clone());
        custom_fields.insert("ros_distro".to_string(), self.ros_distro.clone());
        custom_fields.insert("domain_id".to_string(), self.domain_id.to_string());
        custom_fields.insert("active_topics".to_string(), self.active_topics.join(","));
        custom_fields.insert("active_nodes".to_string(), self.active_nodes.join(","));

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "ros2".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: None,
                memory_usage_percent: None,
                temperature_celsius: None,
                battery_percent: None,
                network_connected: true,
            },
            custom_fields,
        }
    }

    fn get_device_status(&self) -> DeviceStatus {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jetson_heartbeat_provider() {
        let provider = JetsonHeartbeatProvider::new("jetson-001", "Jetson Nano");
        
        assert_eq!(provider.provider_name(), "jetson");
        assert_eq!(provider.device_id(), "jetson-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "jetson");
        assert_eq!(data.custom_fields.get("model").unwrap(), "Jetson Nano");
    }

    #[test]
    fn test_jetson_with_metrics() {
        let provider = JetsonHeartbeatProvider::new("jetson-001", "Jetson Orin")
            .with_gpu_metrics(50.0, 4 * 1024 * 1024 * 1024, 55.0, 15.0)
            .with_cpu_metrics(25.0, 40.0, 50.0);
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("gpu_usage_percent").unwrap(), "50.0");
        assert_eq!(data.custom_fields.get("power_draw").unwrap(), "15.00 W");
    }

    #[test]
    fn test_ros2_heartbeat_provider() {
        let provider = Ros2HeartbeatProvider::new("ros2-001", "camera_node");
        
        assert_eq!(provider.provider_name(), "ros2");
        assert_eq!(provider.device_id(), "ros2-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "ros2");
        assert_eq!(data.custom_fields.get("node_name").unwrap(), "camera_node");
    }

    #[test]
    fn test_ros2_with_ros_info() {
        let provider = Ros2HeartbeatProvider::new("ros2-001", "control_node")
            .with_ros_info("humble", 5);
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("ros_distro").unwrap(), "humble");
        assert_eq!(data.custom_fields.get("domain_id").unwrap(), "5");
    }

    #[test]
    fn test_robot_metrics_default() {
        let metrics = RobotMetrics::default();
        assert!(metrics.node_names.is_empty());
        assert!(metrics.active_topics.is_empty());
    }
}
