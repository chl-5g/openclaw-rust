//! Capability Service - 工具能力服务
//!
//! 提供基于能力的权限控制抽象

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    
    #[error("Invalid capability: {0}")]
    InvalidCapability(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    Http,
    HttpHost(String),
    HttpPath(String),
    Secrets,
    SecretKey(String),
    FsRead,
    FsWrite,
    FsPath(String),
    EnvVar(String),
    EnvPrefix(String),
    Time,
    Random,
}

impl Capability {
    pub fn matches(&self, required: &Capability) -> bool {
        match (self, required) {
            (c, r) if c == r => true,
            (Capability::HttpHost(h), Capability::HttpHost(r)) => {
                if r.starts_with('*') {
                    h.ends_with(&r[1..]) || h == &r[2..]
                } else {
                    h == r
                }
            }
            (Capability::FsPath(p), Capability::FsPath(r)) => {
                p.starts_with(r.trim_end_matches('*'))
            }
            (Capability::HttpPath(p), Capability::HttpPath(r)) => {
                p.starts_with(r.trim_end_matches('*'))
            }
            _ => false,
        }
    }
    
    pub fn requires_http(&self) -> bool {
        matches!(self, Capability::Http | Capability::HttpHost(_) | Capability::HttpPath(_))
    }
    
    pub fn requires_secrets(&self) -> bool {
        matches!(self, Capability::Secrets | Capability::SecretKey(_))
    }
    
    pub fn requires_fs(&self) -> bool {
        matches!(self, Capability::FsRead | Capability::FsWrite | Capability::FsPath(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilityManifest {
    pub tool_id: String,
    pub capabilities: HashSet<Capability>,
    pub description: Option<String>,
}

impl ToolCapabilityManifest {
    pub fn new(tool_id: &str) -> Self {
        Self {
            tool_id: tool_id.to_string(),
            capabilities: HashSet::new(),
            description: None,
        }
    }
    
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.insert(cap);
        self
    }
    
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
    
    pub fn allows_http(&self) -> bool {
        self.capabilities.iter().any(|c| c.requires_http())
    }
    
    pub fn allows_secrets(&self) -> bool {
        self.capabilities.iter().any(|c| c.requires_secrets())
    }
    
    pub fn allows_fs(&self) -> bool {
        self.capabilities.iter().any(|c| c.requires_fs())
    }
}

#[async_trait]
pub trait CapabilityService: Send + Sync {
    async fn register_manifest(&self, manifest: ToolCapabilityManifest) -> Result<(), CapabilityError>;
    
    async fn has_capability(&self, tool_id: &str, cap: &Capability) -> bool;
    
    async fn grant_capability(&self, tool_id: &str, cap: Capability) -> Result<(), CapabilityError>;
    
    async fn revoke_capability(&self, tool_id: &str, cap: &Capability) -> Result<(), CapabilityError>;
    
    async fn get_capabilities(&self, tool_id: &str) -> Result<Vec<Capability>, CapabilityError>;
    
    async fn get_manifest(&self, tool_id: &str) -> Result<Option<ToolCapabilityManifest>, CapabilityError>;
    
    async fn list_tools(&self) -> Result<Vec<String>, CapabilityError>;
    
    async fn remove_tool(&self, tool_id: &str) -> Result<(), CapabilityError>;
}

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MemoryCapabilityService {
    manifests: Arc<RwLock<HashMap<String, ToolCapabilityManifest>>>,
}

impl Default for MemoryCapabilityService {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCapabilityService {
    pub fn new() -> Self {
        Self {
            manifests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn with_manifests(manifests: HashMap<String, ToolCapabilityManifest>) -> Self {
        Self {
            manifests: Arc::new(RwLock::new(manifests)),
        }
    }
}

#[async_trait]
impl CapabilityService for MemoryCapabilityService {
    async fn register_manifest(&self, manifest: ToolCapabilityManifest) -> Result<(), CapabilityError> {
        let mut manifests = self.manifests.write().await;
        manifests.insert(manifest.tool_id.clone(), manifest);
        Ok(())
    }
    
    async fn has_capability(&self, tool_id: &str, cap: &Capability) -> bool {
        let manifests = self.manifests.read().await;
        if let Some(manifest) = manifests.get(tool_id) {
            manifest.capabilities.iter().any(|c| c.matches(cap))
        } else {
            false
        }
    }
    
    async fn grant_capability(&self, tool_id: &str, cap: Capability) -> Result<(), CapabilityError> {
        let mut manifests = self.manifests.write().await;
        if let Some(manifest) = manifests.get_mut(tool_id) {
            manifest.capabilities.insert(cap);
            Ok(())
        } else {
            let manifest = ToolCapabilityManifest::new(tool_id)
                .with_capability(cap);
            manifests.insert(tool_id.to_string(), manifest);
            Ok(())
        }
    }
    
    async fn revoke_capability(&self, tool_id: &str, cap: &Capability) -> Result<(), CapabilityError> {
        let mut manifests = self.manifests.write().await;
        if let Some(manifest) = manifests.get_mut(tool_id) {
            manifest.capabilities.retain(|c| !c.matches(cap));
            Ok(())
        } else {
            Err(CapabilityError::ToolNotFound(tool_id.to_string()))
        }
    }
    
    async fn get_capabilities(&self, tool_id: &str) -> Result<Vec<Capability>, CapabilityError> {
        let manifests = self.manifests.read().await;
        if let Some(manifest) = manifests.get(tool_id) {
            Ok(manifest.capabilities.iter().cloned().collect())
        } else {
            Err(CapabilityError::ToolNotFound(tool_id.to_string()))
        }
    }
    
    async fn get_manifest(&self, tool_id: &str) -> Result<Option<ToolCapabilityManifest>, CapabilityError> {
        let manifests = self.manifests.read().await;
        Ok(manifests.get(tool_id).cloned())
    }
    
    async fn list_tools(&self) -> Result<Vec<String>, CapabilityError> {
        let manifests = self.manifests.read().await;
        Ok(manifests.keys().cloned().collect())
    }
    
    async fn remove_tool(&self, tool_id: &str) -> Result<(), CapabilityError> {
        let mut manifests = self.manifests.write().await;
        manifests.remove(tool_id);
        Ok(())
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capability_exact_match() {
        let cap = Capability::Http;
        let required = Capability::Http;
        assert!(cap.matches(&required));
    }
    
    #[test]
    fn test_capability_wildcard_match() {
        let cap = Capability::HttpHost("api.github.com".to_string());
        let required = Capability::HttpHost("*.github.com".to_string());
        assert!(cap.matches(&required));
    }
    
    #[test]
    fn test_capability_fs_path_match() {
        let cap = Capability::FsPath("/workspace/data/file.txt".to_string());
        let required = Capability::FsPath("/workspace/*".to_string());
        assert!(cap.matches(&required));
    }
    
    #[test]
    fn test_capability_no_match() {
        let cap = Capability::Http;
        let required = Capability::Secrets;
        assert!(!cap.matches(&required));
    }
    
    #[test]
    fn test_manifest_allows_http() {
        let manifest = ToolCapabilityManifest::new("test")
            .with_capability(Capability::Http);
        assert!(manifest.allows_http());
    }
    
    #[test]
    fn test_manifest_allows_secrets() {
        let manifest = ToolCapabilityManifest::new("test")
            .with_capability(Capability::Secrets);
        assert!(manifest.allows_secrets());
    }
    
    #[tokio::test]
    async fn test_register_and_get_manifest() {
        let service = MemoryCapabilityService::new();
        let mut caps = HashSet::new();
        caps.insert(Capability::Http);
        
        let manifest = ToolCapabilityManifest {
            tool_id: "test-tool".to_string(),
            capabilities: caps,
            description: Some("Test tool".to_string()),
        };
        
        service.register_manifest(manifest).await.unwrap();
        
        let result = service.get_manifest("test-tool").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool_id, "test-tool");
    }
    
    #[tokio::test]
    async fn test_has_capability() {
        let service = MemoryCapabilityService::new();
        let mut caps = HashSet::new();
        caps.insert(Capability::Http);
        caps.insert(Capability::HttpHost("api.example.com".to_string()));
        
        let manifest = ToolCapabilityManifest {
            tool_id: "test-tool".to_string(),
            capabilities: caps,
            description: None,
        };
        
        service.register_manifest(manifest).await.unwrap();
        
        assert!(service.has_capability("test-tool", &Capability::Http).await);
        assert!(service.has_capability(
            "test-tool", 
            &Capability::HttpHost("api.example.com".to_string())
        ).await);
        assert!(!service.has_capability("test-tool", &Capability::Secrets).await);
    }
    
    #[tokio::test]
    async fn test_grant_capability() {
        let service = MemoryCapabilityService::new();
        
        service.grant_capability("test-tool", Capability::Http).await.unwrap();
        
        assert!(service.has_capability("test-tool", &Capability::Http).await);
    }
    
    #[tokio::test]
    async fn test_list_tools() {
        let service = MemoryCapabilityService::new();
        
        service.grant_capability("tool1", Capability::Http).await.unwrap();
        service.grant_capability("tool2", Capability::Secrets).await.unwrap();
        
        let tools = service.list_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
    }
}
