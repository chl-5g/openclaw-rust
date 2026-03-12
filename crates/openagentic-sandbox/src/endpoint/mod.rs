//! Endpoint Allowlist Service - 端点白名单服务
//!
//! 提供 HTTP 端点白名单控制

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EndpointError {
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
    
    #[error("Endpoint not allowed: {0}")]
    NotAllowed(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AllowPolicy {
    #[default]
    AllowAll,
    DenyAll,
    Allowlisted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndpointPattern {
    Exact(String),
    Prefix(String),
    Regex(String),
    Wildcard,
}

impl EndpointPattern {
    pub fn matches(&self, path: &str) -> bool {
        match self {
            EndpointPattern::Exact(s) => path == s,
            EndpointPattern::Prefix(p) => path.starts_with(p),
            EndpointPattern::Regex(r) => {
                Regex::new(r)
                    .map(|re: Regex| re.is_match(path))
                    .unwrap_or(false)
            }
            EndpointPattern::Wildcard => true,
        }
    }
    
    pub fn exact(path: &str) -> Self {
        EndpointPattern::Exact(path.to_string())
    }
    
    pub fn prefix(path: &str) -> Self {
        EndpointPattern::Prefix(path.to_string())
    }
    
    pub fn regex(pattern: &str) -> Self {
        EndpointPattern::Regex(pattern.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRule {
    pub host: String,
    pub patterns: Vec<EndpointPattern>,
    pub allowed_methods: HashSet<String>,
    pub description: Option<String>,
}

impl EndpointRule {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            patterns: Vec::new(),
            allowed_methods: ["GET", "POST", "PUT", "DELETE"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            description: None,
        }
    }
    
    pub fn allow_path(mut self, pattern: EndpointPattern) -> Self {
        self.patterns.push(pattern);
        self
    }
    
    pub fn allow_method(mut self, method: &str) -> Self {
        self.allowed_methods.clear();
        self.allowed_methods.insert(method.to_uppercase());
        self
    }
    
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
    
    pub fn allows(&self, host: &str, path: &str, method: &str) -> bool {
        if self.host != host && self.host != "*" {
            return false;
        }
        
        if !self.allowed_methods.contains(&method.to_uppercase()) {
            return false;
        }
        
        self.patterns.iter().any(|p| p.matches(path))
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub host: String,
    pub path: String,
    pub method: String,
}

impl HttpRequest {
    pub fn new(host: &str, path: &str, method: &str) -> Self {
        Self {
            host: host.to_string(),
            path: path.to_string(),
            method: method.to_uppercase(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EndpointDecision {
    pub allowed: bool,
    pub host: String,
    pub path: String,
    pub reason: Option<String>,
}

impl EndpointDecision {
    pub fn allowed(host: &str, path: &str) -> Self {
        Self {
            allowed: true,
            host: host.to_string(),
            path: path.to_string(),
            reason: None,
        }
    }
    
    pub fn denied(host: &str, path: &str, reason: &str) -> Self {
        Self {
            allowed: false,
            host: host.to_string(),
            path: path.to_string(),
            reason: Some(reason.to_string()),
        }
    }
}

#[async_trait]
pub trait EndpointAllowlist: Send + Sync {
    async fn add_rule(&self, rule: EndpointRule) -> Result<(), EndpointError>;
    
    async fn remove_rule(&self, host: &str) -> Result<(), EndpointError>;
    
    async fn check(&self, request: &HttpRequest) -> EndpointDecision;
    
    async fn set_default_policy(&self, policy: AllowPolicy) -> Result<(), EndpointError>;
    
    async fn get_default_policy(&self) -> AllowPolicy;
    
    async fn list_rules(&self) -> Vec<EndpointRule>;
}

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct MemoryEndpointAllowlist {
    rules: Arc<RwLock<HashMap<String, EndpointRule>>>,
    default_policy: Arc<RwLock<AllowPolicy>>,
}

impl Default for MemoryEndpointAllowlist {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryEndpointAllowlist {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            default_policy: Arc::new(RwLock::new(AllowPolicy::DenyAll)),
        }
    }
    
    pub fn with_rules(rules: HashMap<String, EndpointRule>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(rules)),
            default_policy: Arc::new(RwLock::new(AllowPolicy::DenyAll)),
        }
    }
}

#[async_trait]
impl EndpointAllowlist for MemoryEndpointAllowlist {
    async fn add_rule(&self, rule: EndpointRule) -> Result<(), EndpointError> {
        let mut rules = self.rules.write().await;
        rules.insert(rule.host.clone(), rule);
        Ok(())
    }
    
    async fn remove_rule(&self, host: &str) -> Result<(), EndpointError> {
        let mut rules = self.rules.write().await;
        rules.remove(host);
        Ok(())
    }
    
    async fn check(&self, request: &HttpRequest) -> EndpointDecision {
        let rules = self.rules.read().await;
        let default_policy = *self.default_policy.read().await;
        
        if let Some(rule) = rules.get(&request.host) {
            if rule.allows(&request.host, &request.path, &request.method) {
                return EndpointDecision::allowed(&request.host, &request.path);
            }
        }
        
        for rule in rules.values() {
            if rule.allows(&request.host, &request.path, &request.method) {
                return EndpointDecision::allowed(&request.host, &request.path);
            }
        }
        
        match default_policy {
            AllowPolicy::AllowAll => EndpointDecision::allowed(&request.host, &request.path),
            AllowPolicy::DenyAll | AllowPolicy::Allowlisted => {
                EndpointDecision::denied(
                    &request.host, 
                    &request.path,
                    &format!("Host '{}' not in allowlist", request.host)
                )
            }
        }
    }
    
    async fn set_default_policy(&self, policy: AllowPolicy) -> Result<(), EndpointError> {
        let mut default_policy = self.default_policy.write().await;
        *default_policy = policy;
        Ok(())
    }
    
    async fn get_default_policy(&self) -> AllowPolicy {
        *self.default_policy.read().await
    }
    
    async fn list_rules(&self) -> Vec<EndpointRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_endpoint_pattern_exact() {
        let pattern = EndpointPattern::exact("/api/users");
        assert!(pattern.matches("/api/users"));
        assert!(!pattern.matches("/api/posts"));
    }
    
    #[test]
    fn test_endpoint_pattern_prefix() {
        let pattern = EndpointPattern::prefix("/api/");
        assert!(pattern.matches("/api/users"));
        assert!(pattern.matches("/api/posts/123"));
        assert!(!pattern.matches("/admin/users"));
    }
    
    #[test]
    fn test_endpoint_pattern_wildcard() {
        let pattern = EndpointPattern::Wildcard;
        assert!(pattern.matches("/any/path"));
        assert!(pattern.matches("/"));
    }
    #[tokio::test]
    async fn test_endpoint_rule_allows() {
        let rule = EndpointRule::new("api.example.com")
            .allow_path(EndpointPattern::prefix("/api/"))
            .allow_method("GET");
        
        assert!(rule.allows("api.example.com", "/api/users", "GET"));
        assert!(!rule.allows("api.example.com", "/api/users", "POST"));
        assert!(!rule.allows("other.com", "/api/users", "GET"));
    }
    
    #[tokio::test]
    async fn test_add_and_check_rule() {
        let allowlist = MemoryEndpointAllowlist::new();
        
        allowlist.add_rule(
            EndpointRule::new("api.example.com")
                .allow_path(EndpointPattern::Wildcard)
        ).await.unwrap();
        
        let request = HttpRequest::new("api.example.com", "/api/users", "GET");
        let decision = allowlist.check(&request).await;
        
        assert!(decision.allowed);
    }
    
    #[tokio::test]
    async fn test_default_deny() {
        let allowlist = MemoryEndpointAllowlist::new();
        
        let request = HttpRequest::new("unknown.com", "/api/users", "GET");
        let decision = allowlist.check(&request).await;
        
        assert!(!decision.allowed);
    }
    
    #[tokio::test]
    async fn test_default_allow_all() {
        let allowlist = MemoryEndpointAllowlist::new();
        allowlist.set_default_policy(AllowPolicy::AllowAll).await.unwrap();
        
        let request = HttpRequest::new("any.com", "/any/path", "GET");
        let decision = allowlist.check(&request).await;
        
        assert!(decision.allowed);
    }
    
    #[tokio::test]
    async fn test_remove_rule() {
        let allowlist = MemoryEndpointAllowlist::new();
        
        allowlist.add_rule(
            EndpointRule::new("api.example.com")
                .allow_path(EndpointPattern::Wildcard)
        ).await.unwrap();
        
        allowlist.remove_rule("api.example.com").await.unwrap();
        
        let request = HttpRequest::new("api.example.com", "/api/users", "GET");
        let decision = allowlist.check(&request).await;
        
        assert!(!decision.allowed);
    }
    
    #[tokio::test]
    async fn test_list_rules() {
        let allowlist = MemoryEndpointAllowlist::new();
        
        allowlist.add_rule(EndpointRule::new("api1.com").allow_path(EndpointPattern::Wildcard)).await.unwrap();
        allowlist.add_rule(EndpointRule::new("api2.com").allow_path(EndpointPattern::Wildcard)).await.unwrap();
        
        let rules = allowlist.list_rules().await;
        assert_eq!(rules.len(), 2);
    }
}
