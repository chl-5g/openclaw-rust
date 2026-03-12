use std::collections::HashMap;
use std::time::Instant;

use crate::heartbeat::{HeartbeatData, HeartbeatProvider, DeviceStatus, DeviceMetrics, current_timestamp};

pub trait EmbeddedHeartbeatProvider: HeartbeatProvider {
    fn get_firmware_version(&self) -> Option<&str>;
    fn get_free_heap_bytes(&self) -> Option<u32>;
    fn get_wifi_signal_strength(&self) -> Option<i32>;
    fn get_cpu_temperature(&self) -> Option<f32>;
    
    fn get_embedded_metrics(&self) -> EmbeddedMetrics {
        EmbeddedMetrics {
            firmware_version: self.get_firmware_version().map(String::from),
            free_heap_bytes: self.get_free_heap_bytes(),
            wifi_rssi: self.get_wifi_signal_strength(),
            cpu_temperature: self.get_cpu_temperature(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmbeddedMetrics {
    pub firmware_version: Option<String>,
    pub free_heap_bytes: Option<u32>,
    pub wifi_rssi: Option<i32>,
    pub cpu_temperature: Option<f32>,
}

pub struct Esp32HeartbeatProvider {
    id: String,
    chip_model: String,
    chip_revision: u32,
    flash_size: u32,
    wifi_ssid: Option<String>,
    rssi: Option<i32>,
    free_heap: u32,
    firmware_version: String,
    start_time: Instant,
    status: DeviceStatus,
}

impl Esp32HeartbeatProvider {
    pub fn new(id: &str, chip_model: &str) -> Self {
        Self {
            id: id.to_string(),
            chip_model: chip_model.to_string(),
            chip_revision: 3,
            flash_size: 4 * 1024 * 1024,
            wifi_ssid: None,
            rssi: None,
            free_heap: 200 * 1024,
            firmware_version: "1.0.0".to_string(),
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }

    pub fn with_wifi(mut self, ssid: &str, rssi: i32) -> Self {
        self.wifi_ssid = Some(ssid.to_string());
        self.rssi = Some(rssi);
        self
    }

    pub fn with_firmware(mut self, version: &str) -> Self {
        self.firmware_version = version.to_string();
        self
    }
}

impl EmbeddedHeartbeatProvider for Esp32HeartbeatProvider {
    fn get_firmware_version(&self) -> Option<&str> {
        Some(&self.firmware_version)
    }

    fn get_free_heap_bytes(&self) -> Option<u32> {
        Some(self.free_heap)
    }

    fn get_wifi_signal_strength(&self) -> Option<i32> {
        self.rssi
    }

    fn get_cpu_temperature(&self) -> Option<f32> {
        None
    }
}

impl HeartbeatProvider for Esp32HeartbeatProvider {
    fn provider_name(&self) -> &str {
        "esp32"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let embedded_metrics = self.get_embedded_metrics();
        let mut custom_fields = HashMap::new();
        custom_fields.insert("chip_model".to_string(), self.chip_model.clone());
        custom_fields.insert("chip_revision".to_string(), self.chip_revision.to_string());
        custom_fields.insert("flash_size".to_string(), format!("{} bytes", self.flash_size));
        
        if let Some(ref ssid) = self.wifi_ssid {
            custom_fields.insert("wifi_ssid".to_string(), ssid.clone());
        }
        
        if let Some(rssi) = self.rssi {
            custom_fields.insert("wifi_rssi".to_string(), rssi.to_string());
        }

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "esp32".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: None,
                memory_usage_percent: embedded_metrics.free_heap_bytes.map(|h| {
                    let used = (self.flash_size as f32) - (h as f32);
                    (used / (self.flash_size as f32)) * 100.0
                }),
                temperature_celsius: embedded_metrics.cpu_temperature,
                battery_percent: None,
                network_connected: self.wifi_ssid.is_some(),
            },
            custom_fields,
        }
    }

    fn get_device_status(&self) -> DeviceStatus {
        self.status
    }
}

pub struct Stm32HeartbeatProvider {
    id: String,
    model: String,
    clock_speed: u32,
    flash_size: u32,
    flash_used: u32,
    free_heap: u32,
    firmware_version: String,
    start_time: Instant,
    status: DeviceStatus,
}

impl Stm32HeartbeatProvider {
    pub fn new(id: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            model: model.to_string(),
            clock_speed: 168_000_000,
            flash_size: 1024 * 1024,
            flash_used: 256 * 1024,
            free_heap: 64 * 1024,
            firmware_version: "1.0.0".to_string(),
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }
}

impl EmbeddedHeartbeatProvider for Stm32HeartbeatProvider {
    fn get_firmware_version(&self) -> Option<&str> {
        Some(&self.firmware_version)
    }

    fn get_free_heap_bytes(&self) -> Option<u32> {
        Some(self.free_heap)
    }

    fn get_wifi_signal_strength(&self) -> Option<i32> {
        None
    }

    fn get_cpu_temperature(&self) -> Option<f32> {
        None
    }
}

impl HeartbeatProvider for Stm32HeartbeatProvider {
    fn provider_name(&self) -> &str {
        "stm32"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("model".to_string(), self.model.clone());
        custom_fields.insert("clock_speed".to_string(), format!("{} Hz", self.clock_speed));
        custom_fields.insert("flash_size".to_string(), format!("{} bytes", self.flash_size));
        custom_fields.insert("flash_used".to_string(), format!("{} bytes", self.flash_used));

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "stm32".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: None,
                memory_usage_percent: Some(((self.flash_used as f32) / (self.flash_size as f32)) * 100.0),
                temperature_celsius: None,
                battery_percent: None,
                network_connected: false,
            },
            custom_fields,
        }
    }

    fn get_device_status(&self) -> DeviceStatus {
        self.status
    }
}

pub struct RiscVHeartbeatProvider {
    id: String,
    isa_extensions: Vec<String>,
    hart_count: u32,
    vendor_id: String,
    free_heap: u32,
    firmware_version: String,
    start_time: Instant,
    status: DeviceStatus,
}

impl RiscVHeartbeatProvider {
    pub fn new(id: &str, vendor_id: &str) -> Self {
        Self {
            id: id.to_string(),
            isa_extensions: vec!["I".to_string(), "M".to_string(), "A".to_string(), "F".to_string(), "D".to_string()],
            hart_count: 1,
            vendor_id: vendor_id.to_string(),
            free_heap: 512 * 1024,
            firmware_version: "1.0.0".to_string(),
            start_time: Instant::now(),
            status: DeviceStatus::Online,
        }
    }
}

impl EmbeddedHeartbeatProvider for RiscVHeartbeatProvider {
    fn get_firmware_version(&self) -> Option<&str> {
        Some(&self.firmware_version)
    }

    fn get_free_heap_bytes(&self) -> Option<u32> {
        Some(self.free_heap)
    }

    fn get_wifi_signal_strength(&self) -> Option<i32> {
        None
    }

    fn get_cpu_temperature(&self) -> Option<f32> {
        None
    }
}

impl HeartbeatProvider for RiscVHeartbeatProvider {
    fn provider_name(&self) -> &str {
        "riscv"
    }

    fn device_id(&self) -> &str {
        &self.id
    }

    fn get_heartbeat_data(&self) -> HeartbeatData {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("vendor_id".to_string(), self.vendor_id.clone());
        custom_fields.insert("hart_count".to_string(), self.hart_count.to_string());
        custom_fields.insert("isa_extensions".to_string(), self.isa_extensions.join(","));

        HeartbeatData {
            device_id: self.id.clone(),
            provider_name: "riscv".to_string(),
            timestamp: current_timestamp(),
            status: self.status,
            metrics: DeviceMetrics {
                uptime_secs: self.start_time.elapsed().as_secs(),
                cpu_usage_percent: None,
                memory_usage_percent: None,
                temperature_celsius: None,
                battery_percent: None,
                network_connected: false,
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
    fn test_esp32_heartbeat_provider() {
        let provider = Esp32HeartbeatProvider::new("esp32-001", "ESP32-S3");
        
        assert_eq!(provider.provider_name(), "esp32");
        assert_eq!(provider.device_id(), "esp32-001");
        assert_eq!(provider.get_device_status(), DeviceStatus::Online);
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "esp32");
        assert_eq!(data.custom_fields.get("chip_model").unwrap(), "ESP32-S3");
    }

    #[test]
    fn test_esp32_with_wifi() {
        let provider = Esp32HeartbeatProvider::new("esp32-001", "ESP32-S3")
            .with_wifi("MyNetwork", -45);
        
        let data = provider.get_heartbeat_data();
        assert!(data.metrics.network_connected);
        assert_eq!(data.custom_fields.get("wifi_ssid").unwrap(), "MyNetwork");
        assert_eq!(data.custom_fields.get("wifi_rssi").unwrap(), "-45");
    }

    #[test]
    fn test_stm32_heartbeat_provider() {
        let provider = Stm32HeartbeatProvider::new("stm32-001", "STM32F407");
        
        assert_eq!(provider.provider_name(), "stm32");
        assert_eq!(provider.device_id(), "stm32-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "stm32");
        assert_eq!(data.custom_fields.get("model").unwrap(), "STM32F407");
    }

    #[test]
    fn test_riscv_heartbeat_provider() {
        let provider = RiscVHeartbeatProvider::new("riscv-001", "SiFive");
        
        assert_eq!(provider.provider_name(), "riscv");
        assert_eq!(provider.device_id(), "riscv-001");
        
        let data = provider.get_heartbeat_data();
        assert_eq!(data.provider_name, "riscv");
        assert_eq!(data.custom_fields.get("vendor_id").unwrap(), "SiFive");
    }

    #[test]
    fn test_embedded_metrics_default() {
        let metrics = EmbeddedMetrics::default();
        assert!(metrics.firmware_version.is_none());
        assert!(metrics.free_heap_bytes.is_none());
    }
}
