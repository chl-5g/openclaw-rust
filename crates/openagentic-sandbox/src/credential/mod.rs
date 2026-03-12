//! Credential Service - 凭证服务
//!
//! 提供凭证边界注入管理

pub mod vault;
pub mod keyring;
pub mod encrypted_vault;
pub mod audit;
pub mod rotation;

#[cfg(feature = "credential-vault")]
pub use vault::{Vault, VaultError, memory::MemoryVault};

#[cfg(all(feature = "credential-vault", feature = "credential-sqlite"))]
pub use vault::sqlite::SqliteVault;

#[cfg(not(feature = "credential-vault"))]
pub use vault::{Vault, VaultError};

#[cfg(feature = "credential-keyring")]
pub use keyring::{Keyring, KeyringError, system::SystemKeyring};

#[cfg(feature = "credential-vault")]
pub use encrypted_vault::{EncryptedVault, AesKeyGenerator};

#[cfg(feature = "credential-audit")]
pub use audit::{AuditLogger, AuditEntry, AuditAction, AuditResult, InMemoryAuditLogger};

#[cfg(feature = "credential-rotation")]
pub use rotation::{CredentialRotator, RotationResult};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Credential not found: {0}")]
    NotFound(String),
    
    #[error("Credential denied: {0}")]
    AccessDenied(String),
    
    #[error("Invalid credential: {0}")]
    InvalidCredential(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub key: String,
    pub value: String,
    pub hash: String,
}

impl Credential {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        let value = value.into();
        let hash = Self::hash_value(&key, &value);
        Self { key, value, hash }
    }
    
    fn hash_value(key: &str, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(b":");
        hasher.update(value.as_bytes());
        let result = hasher.finalize();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result)
    }
    
    pub fn mask(&self) -> String {
        if self.value.len() <= 4 {
            "*".repeat(self.value.len())
        } else {
            format!("{}...{}", &self.value[..2], &self.value[self.value.len()-2..])
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSet {
    pub name: String,
    pub credentials: Vec<Credential>,
    pub metadata: HashMap<String, String>,
}

impl CredentialSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            credentials: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn add_credential(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.credentials.push(Credential::new(key, value));
        self
    }
    
    pub fn get(&self, key: &str) -> Option<&Credential> {
        self.credentials.iter().find(|c| c.key == key)
    }
    
    pub fn inject_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        for cred in &self.credentials {
            env.insert(cred.key.clone(), cred.value.clone());
        }
        env
    }
    
    pub fn inject_env_with_prefix(&self, prefix: &str) -> HashMap<String, String> {
        let mut env = HashMap::new();
        for cred in &self.credentials {
            env.insert(format!("{}_{}", prefix, cred.key), cred.value.clone());
        }
        env
    }
}

#[async_trait]
pub trait CredentialService: Send + Sync {
    async fn create_credential_set(&self, set: CredentialSet) -> Result<(), CredentialError>;
    
    async fn get_credential_set(&self, name: &str) -> Result<Option<CredentialSet>, CredentialError>;
    
    async fn delete_credential_set(&self, name: &str) -> Result<(), CredentialError>;
    
    async fn inject_credentials(&self, set_name: &str, env: &mut HashMap<String, String>) -> Result<(), CredentialError>;
    
    async fn list_sets(&self) -> Result<Vec<String>, CredentialError>;
    
    async fn add_credential(&self, set_name: &str, key: String, value: String) -> Result<(), CredentialError>;
    
    async fn remove_credential(&self, set_name: &str, key: &str) -> Result<(), CredentialError>;
    
    async fn check_access(&self, set_name: &str, tool_id: &str) -> Result<bool, CredentialError>;
    
    async fn grant_access(&self, set_name: &str, tool_id: &str) -> Result<(), CredentialError>;
    
    async fn revoke_access(&self, set_name: &str, tool_id: &str) -> Result<(), CredentialError>;
}

pub struct MemoryCredentialService {
    sets: Arc<RwLock<HashMap<String, CredentialSet>>>,
    access_control: Arc<RwLock<HashMap<String, HashMap<String, bool>>>>,
}

impl Default for MemoryCredentialService {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCredentialService {
    pub fn new() -> Self {
        Self {
            sets: Arc::new(RwLock::new(HashMap::new())),
            access_control: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn with_credentials(sets: HashMap<String, CredentialSet>) -> Self {
        Self {
            sets: Arc::new(RwLock::new(sets)),
            access_control: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CredentialService for MemoryCredentialService {
    async fn create_credential_set(&self, set: CredentialSet) -> Result<(), CredentialError> {
        let mut sets = self.sets.write().await;
        sets.insert(set.name.clone(), set);
        Ok(())
    }
    
    async fn get_credential_set(&self, name: &str) -> Result<Option<CredentialSet>, CredentialError> {
        let sets = self.sets.read().await;
        Ok(sets.get(name).cloned())
    }
    
    async fn delete_credential_set(&self, name: &str) -> Result<(), CredentialError> {
        let mut sets = self.sets.write().await;
        sets.remove(name);
        Ok(())
    }
    
    async fn inject_credentials(&self, set_name: &str, env: &mut HashMap<String, String>) -> Result<(), CredentialError> {
        let sets = self.sets.read().await;
        if let Some(set) = sets.get(set_name) {
            for cred in &set.credentials {
                env.insert(cred.key.clone(), cred.value.clone());
            }
            Ok(())
        } else {
            Err(CredentialError::NotFound(set_name.to_string()))
        }
    }
    
    async fn list_sets(&self) -> Result<Vec<String>, CredentialError> {
        let sets = self.sets.read().await;
        Ok(sets.keys().cloned().collect())
    }
    
    async fn add_credential(&self, set_name: &str, key: String, value: String) -> Result<(), CredentialError> {
        let mut sets = self.sets.write().await;
        if let Some(set) = sets.get_mut(set_name) {
            set.credentials.push(Credential::new(key, value));
            Ok(())
        } else {
            Err(CredentialError::NotFound(set_name.to_string()))
        }
    }
    
    async fn remove_credential(&self, set_name: &str, key: &str) -> Result<(), CredentialError> {
        let mut sets = self.sets.write().await;
        if let Some(set) = sets.get_mut(set_name) {
            set.credentials.retain(|c| c.key != key);
            Ok(())
        } else {
            Err(CredentialError::NotFound(set_name.to_string()))
        }
    }
    
    async fn check_access(&self, set_name: &str, tool_id: &str) -> Result<bool, CredentialError> {
        let access = self.access_control.read().await;
        if let Some(tools) = access.get(set_name) {
            Ok(tools.get(tool_id).copied().unwrap_or(false))
        } else {
            Ok(false)
        }
    }
    
    async fn grant_access(&self, set_name: &str, tool_id: &str) -> Result<(), CredentialError> {
        let mut access = self.access_control.write().await;
        access.entry(set_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(tool_id.to_string(), true);
        Ok(())
    }
    
    async fn revoke_access(&self, set_name: &str, tool_id: &str) -> Result<(), CredentialError> {
        let mut access = self.access_control.write().await;
        if let Some(tools) = access.get_mut(set_name) {
            tools.remove(tool_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credential_creation() {
        let cred = Credential::new("API_KEY", "secret-value-123");
        assert_eq!(cred.key, "API_KEY");
        assert_eq!(cred.value, "secret-value-123");
    }
    
    #[tokio::test]
    async fn test_credential_mask() {
        let cred = Credential::new("KEY", "short");
        assert_eq!(cred.mask(), "sh...rt");
        
        let cred = Credential::new("KEY", "longer-secret-value");
        assert_eq!(cred.mask(), "lo...ue");
    }
    
    #[test]
    fn test_credential_set_inject() {
        let set = CredentialSet::new("test")
            .add_credential("KEY1", "value1")
            .add_credential("KEY2", "value2");
        
        let env = set.inject_env();
        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
    }
    
    #[test]
    fn test_credential_set_inject_with_prefix() {
        let set = CredentialSet::new("test")
            .add_credential("KEY", "value");
        
        let env = set.inject_env_with_prefix("MYAPP");
        assert_eq!(env.get("MYAPP_KEY"), Some(&"value".to_string()));
    }
    
    #[tokio::test]
    async fn test_create_and_get_credential_set() {
        let service = MemoryCredentialService::new();
        let set = CredentialSet::new("test-set")
            .add_credential("API_KEY", "secret");
        
        service.create_credential_set(set).await.unwrap();
        
        let result = service.get_credential_set("test-set").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().credentials.len(), 1);
    }
    
    #[tokio::test]
    async fn test_inject_credentials() {
        let service = MemoryCredentialService::new();
        let set = CredentialSet::new("test-set")
            .add_credential("API_KEY", "secret-value");
        
        service.create_credential_set(set).await.unwrap();
        
        let mut env = HashMap::new();
        service.inject_credentials("test-set", &mut env).await.unwrap();
        
        assert_eq!(env.get("API_KEY"), Some(&"secret-value".to_string()));
    }
    
    #[tokio::test]
    async fn test_list_sets() {
        let service = MemoryCredentialService::new();
        
        service.create_credential_set(CredentialSet::new("set1")).await.unwrap();
        service.create_credential_set(CredentialSet::new("set2")).await.unwrap();
        
        let sets = service.list_sets().await.unwrap();
        assert_eq!(sets.len(), 2);
    }
    
    #[tokio::test]
    async fn test_grant_and_check_access() {
        let service = MemoryCredentialService::new();
        
        service.create_credential_set(CredentialSet::new("test-set")).await.unwrap();
        service.grant_access("test-set", "tool-1").await.unwrap();
        
        let has_access = service.check_access("test-set", "tool-1").await.unwrap();
        assert!(has_access);
        
        let no_access = service.check_access("test-set", "tool-2").await.unwrap();
        assert!(!no_access);
    }
    
    #[tokio::test]
    async fn test_revoke_access() {
        let service = MemoryCredentialService::new();
        
        service.create_credential_set(CredentialSet::new("test-set")).await.unwrap();
        service.grant_access("test-set", "tool-1").await.unwrap();
        service.revoke_access("test-set", "tool-1").await.unwrap();
        
        let no_access = service.check_access("test-set", "tool-1").await.unwrap();
        assert!(!no_access);
    }
}
