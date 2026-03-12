//! Credential Rotation Module - 凭证轮换
//!
//! 提供凭证自动轮换功能

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RotationError {
    #[error("Rotation failed: {0}")]
    Failed(String),
    
    #[error("Credential not found: {0}")]
    NotFound(String),
    
    #[error("Rotation not scheduled: {0}")]
    NotScheduled(String),
    
    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    pub old_key: String,
    pub new_key: String,
    pub rotated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStrategy {
    Manual,
    TimeBased,
    UsageBased,
    AlertBased,
}

pub trait CredentialRotator: Send + Sync {
    fn rotate(&self, key: &str) -> Result<RotationResult, RotationError>;
    
    fn schedule_rotation(
        &self,
        key: &str,
        interval: Duration,
    ) -> Result<(), RotationError>;
    
    fn cancel_rotation(&self, key: &str) -> Result<(), RotationError>;
    
    fn get_next_rotation(&self, key: &str) -> Option<DateTime<Utc>>;
    
    fn list_scheduled_rotations(&self) -> Vec<(String, DateTime<Utc>)>;
}

pub struct InMemoryCredentialRotator {
    schedules: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    history: Arc<Mutex<Vec<RotationResult>>>,
}

impl Default for InMemoryCredentialRotator {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCredentialRotator {
    pub fn new() -> Self {
        Self {
            schedules: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn with_rotation_callback<F>(self, _callback: F) -> Self
    where
        F: Fn(String, String) + Send + Sync + 'static,
    {
        self
    }
}

impl CredentialRotator for InMemoryCredentialRotator {
    fn rotate(&self, key: &str) -> Result<RotationResult, RotationError> {
        let new_value = Self::generate_new_value();
        
        let result = RotationResult {
            old_key: key.to_string(),
            new_key: new_value.clone(),
            rotated_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(90),
        };
        
        let history = self.history.clone();
        
        if let Ok(mut guard) = history.lock() {
            guard.push(RotationResult {
                old_key: key.to_string(),
                new_key: new_value,
                rotated_at: result.rotated_at,
                expires_at: result.expires_at,
            });
        }
        
        Ok(result)
    }
    
    fn schedule_rotation(
        &self,
        key: &str,
        interval: Duration,
    ) -> Result<(), RotationError> {
        if interval <= Duration::zero() {
            return Err(RotationError::InvalidSchedule(
                "Interval must be positive".to_string(),
            ));
        }
        
        let next_rotation = Utc::now() + interval;
        
        let schedules = self.schedules.clone();
        
        if let Ok(mut guard) = schedules.lock() {
            guard.insert(key.to_string(), next_rotation);
        }
        
        Ok(())
    }
    
    fn cancel_rotation(&self, key: &str) -> Result<(), RotationError> {
        let schedules = self.schedules.clone();
        
        if let Ok(mut guard) = schedules.lock() {
            guard.remove(key);
        }
        
        Ok(())
    }
    
    fn get_next_rotation(&self, key: &str) -> Option<DateTime<Utc>> {
        let schedules = self.schedules.clone();
        
        if let Ok(guard) = schedules.lock() {
            return guard.get(key).copied();
        }
        None
    }
    
    fn list_scheduled_rotations(&self) -> Vec<(String, DateTime<Utc>)> {
        let schedules = self.schedules.clone();
        
        if let Ok(guard) = schedules.lock() {
            return guard.iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
        }
        Vec::new()
    }
}

impl InMemoryCredentialRotator {
    fn generate_new_value() -> String {
        use rand::Rng;
        use base64::Engine;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen::<u8>()).collect();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    }
}

#[cfg(feature = "credential-rotation")]
pub mod scheduler {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;
    
    pub struct RotationScheduler {
        rotator: Arc<dyn CredentialRotator>,
        running: Arc<Mutex<bool>>,
    }
    
    impl RotationScheduler {
        pub fn new(rotator: Arc<dyn CredentialRotator>) -> Self {
            Self {
                rotator,
                running: Arc::new(Mutex::new(false)),
            }
        }
        
        pub fn start(&self) {
            *self.running.lock().unwrap() = true;
            
            let rotator = self.rotator.clone();
            let running = self.running.clone();
            
            thread::spawn(move || {
                loop {
                    thread::sleep(StdDuration::from_secs(3600));
                    
                    if !*running.lock().unwrap() {
                        break;
                    }
                    
                    let scheduled = rotator.list_scheduled_rotations();
                    let now = Utc::now();
                    
                    for (key, next_rotation) in scheduled {
                        if now >= next_rotation {
                            match rotator.rotate(&key) {
                                Ok(result) => {
                                    tracing::info!(
                                        "Rotated credential {}: expires at {}",
                                        key,
                                        result.expires_at
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to rotate credential {}: {}",
                                        key,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            });
        }
        
        pub fn stop(&self) {
            *self.running.lock().unwrap() = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rotation_result_creation() {
        let result = RotationResult {
            old_key: "old_key".to_string(),
            new_key: "new_key".to_string(),
            rotated_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(90),
        };
        
        assert_eq!(result.old_key, "old_key");
        assert_eq!(result.new_key, "new_key");
    }
    
    #[test]
    fn test_schedule_rotation() {
        let rotator = InMemoryCredentialRotator::new();
        
        let result = rotator.schedule_rotation(
            "api_key",
            Duration::days(30),
        );
        
        assert!(result.is_ok());
        
        let next = rotator.get_next_rotation("api_key");
        assert!(next.is_some());
    }
    
    #[test]
    fn test_cancel_rotation() {
        let rotator = InMemoryCredentialRotator::new();
        
        rotator.schedule_rotation("api_key", Duration::days(30)).unwrap();
        rotator.cancel_rotation("api_key").unwrap();
        
        let next = rotator.get_next_rotation("api_key");
        assert!(next.is_none());
    }
    
    #[test]
    fn test_list_scheduled_rotations() {
        let rotator = InMemoryCredentialRotator::new();
        
        rotator.schedule_rotation("key1", Duration::days(30)).unwrap();
        rotator.schedule_rotation("key2", Duration::days(60)).unwrap();
        
        let scheduled = rotator.list_scheduled_rotations();
        assert_eq!(scheduled.len(), 2);
    }
    
    #[test]
    fn test_invalid_schedule() {
        let rotator = InMemoryCredentialRotator::new();
        
        let result = rotator.schedule_rotation(
            "api_key",
            Duration::zero(),
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_rotate() {
        let rotator = InMemoryCredentialRotator::new();
        
        let result = rotator.rotate("api_key");
        
        assert!(result.is_ok());
        let rotation = result.unwrap();
        assert_ne!(rotation.old_key, rotation.new_key);
    }
}
