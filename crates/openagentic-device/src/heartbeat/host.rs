use std::collections::HashMap;
use std::time::Instant;

use crate::heartbeat::{HeartbeatData, HeartbeatProvider, DeviceStatus, DeviceMetrics, current_timestamp};

pub struct CameraHeartbeatProvider {
    id: String,
    resolution: String,
    fps: u32,
    start_time: Instant,
    status: DeviceStatus,
}

impl CameraHeartbeatProvider {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            resolution: "1920x1080".to_string(),
            fps: 30,
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_config(mut self, resolution: &str, fps: u32) -> Self {
        self.resolution = resolution.to_string();
        self.fps = fps;
        self
    }
}

impl HeartbeatProvider for CameraHeartbeatProvider {
    fn provider_name(&self) -> &str {
        "camera"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("resolution".to_string(), self.resolution.clone());
        custom_fields.insert("fps".to_string(), self.fps.to_string());

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "camera".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: Some(15.0),
                memory_usage_percent: Some(25.0),
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

pub struct ScreenHeartbeatProvider {
    id: String,
    width: u32,
    height: u32,
    is_recording: bool,
    start_time: Instant,
    status: DeviceStatus,
}

impl ScreenHeartbeatProvider {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            width: 1920,
            height: 1080,
            is_recording: false,
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn set_recording(&mut self, recording: bool) {
        self.is_recording = recording;
    }
}

impl HeartbeatProvider for ScreenHeartbeatProvider {
    fn provider_name(&self) -> &str {
        "screen"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("resolution".to_string(), format!("{}x{}", self.width, self.height));
        custom_fields.insert("is_recording".to_string(), self.is_recording.to_string());

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "screen".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: if self.is_recording { Some(30.0) } else { Some(5.0) },
                memory_usage_percent: Some(20.0),
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

pub struct LocationHeartbeatProvider {
    id: String,
    provider: String,
    accuracy: f32,
    altitude: Option<f64>,
    start_time: Instant,
    status: DeviceStatus,
}

impl LocationHeartbeatProvider {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            provider: "gps".to_string(),
            accuracy: 5.0,
            altitude: None,
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_provider(mut self, provider: &str) -> Self {
        self.provider = provider.to_string();
        self
    }

    pub fn with_position(mut self, accuracy: f32, altitude: Option<f64>) -> Self {
        self.accuracy = accuracy;
        self.altitude = altitude;
        self
    }
}

impl HeartbeatProvider for LocationHeartbeatProvider {
    fn provider_name(&self) -> &str {
        "location"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("provider".to_string(), self.provider.clone());
        custom_fields.insert("accuracy".to_string(), format!("{:.1}m", self.accuracy));
        
        if let Some(alt) = self.altitude {
            custom_fields.insert("altitude".to_string(), format!("{:.1}m", alt));
        }

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "location".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: Some(5.0),
                memory_usage_percent: Some(10.0),
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
    fn test_camera_heartbeat_provider() {
        let provider = CameraHeartbeatProvider::new("cam-001");
        
        assert_eq!(provider.provider_name(), "camera");
        assert_eq!(provider.device_id(), "cam-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "camera");
        assert_eq!(data.custom_fields.get("resolution").unwrap(), "1920x1080");
    }

    #[test]
    fn test_camera_with_config() {
        let provider = CameraHeartbeatProvider::new("cam-001")
            .with_config("4K", 60);
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("resolution").unwrap(), "4K");
        assert_eq!(data.custom_fields.get("fps").unwrap(), "60");
    }

    #[test]
    fn test_screen_heartbeat_provider() {
        let provider = ScreenHeartbeatProvider::new("screen-001");
        
        assert_eq!(provider.provider_name(), "screen");
        assert_eq!(provider.device_id(), "screen-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "screen");
        assert_eq!(data.custom_fields.get("resolution").unwrap(), "1920x1080");
    }

    #[test]
    fn test_screen_recording_state() {
        let mut provider = ScreenHeartbeatProvider::new("screen-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("is_recording").unwrap(), "false");
        
        provider.set_recording(true);
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("is_recording").unwrap(), "true");
    }

    #[test]
    fn test_location_heartbeat_provider() {
        let provider = LocationHeartbeatProvider::new("loc-001");
        
        assert_eq!(provider.provider_name(), "location");
        assert_eq!(provider.device_id(), "loc-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "location");
        assert_eq!(data.custom_fields.get("provider").unwrap(), "gps");
    }

    #[test]
    fn test_location_with_position() {
        let provider = LocationHeartbeatProvider::new("loc-001")
            .with_provider("gps")
            .with_position(3.5, Some(120.5));
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.custom_fields.get("accuracy").unwrap(), "3.5m");
        assert_eq!(data.custom_fields.get("altitude").unwrap(), "120.5m");
    }
}
