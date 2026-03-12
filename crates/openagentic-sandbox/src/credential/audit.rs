//! Audit Module - 审计日志
//!
//! 提供凭证访问审计功能

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Write,
    Delete,
    Inject,
    Encrypt,
    Decrypt,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Create => write!(f, "CREATE"),
            AuditAction::Read => write!(f, "READ"),
            AuditAction::Update => write!(f, "UPDATE"),
            AuditAction::Write => write!(f, "WRITE"),
            AuditAction::Delete => write!(f, "DELETE"),
            AuditAction::Inject => write!(f, "INJECT"),
            AuditAction::Encrypt => write!(f, "ENCRYPT"),
            AuditAction::Decrypt => write!(f, "DECRYPT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

impl std::fmt::Display for AuditResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditResult::Success => write!(f, "SUCCESS"),
            AuditResult::Failure => write!(f, "FAILURE"),
            AuditResult::Denied => write!(f, "DENIED"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub credential_key: String,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub result: AuditResult,
    pub details: Option<String>,
}

impl AuditEntry {
    pub fn new(
        action: AuditAction,
        credential_key: String,
        result: AuditResult,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            action,
            credential_key,
            user_id: None,
            ip_address: None,
            result,
            details: None,
        }
    }
    
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }
    
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

pub trait AuditLogger: Send + Sync {
    fn log(&self, entry: AuditEntry);
    
    fn query(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        action: Option<AuditAction>,
        credential_key: Option<&str>,
    ) -> Vec<AuditEntry>;
}

pub struct InMemoryAuditLogger {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    max_entries: usize,
}

impl Default for InMemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAuditLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000,
        }
    }
    
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }
}

impl AuditLogger for InMemoryAuditLogger {
    fn log(&self, entry: AuditEntry) {
        if let Ok(mut guard) = self.entries.write() {
            guard.push(entry);
            
            if guard.len() > self.max_entries {
                let remove_count = guard.len() - self.max_entries;
                guard.drain(0..remove_count);
            }
        }
    }
    
    fn query(
        &self,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        action: Option<AuditAction>,
        credential_key: Option<&str>,
    ) -> Vec<AuditEntry> {
        if let Ok(guard) = self.entries.read() {
            return guard
                .iter()
                .filter(|e| {
                    if let Some(start) = start_time {
                        if e.timestamp < start {
                            return false;
                        }
                    }
                    
                    if let Some(end) = end_time {
                        if e.timestamp > end {
                            return false;
                        }
                    }
                    
                    if let Some(act) = action {
                        if e.action != act {
                            return false;
                        }
                    }
                    
                    if let Some(key) = credential_key {
                        if &e.credential_key != key {
                            return false;
                        }
                    }
                    
                    true
                })
                .cloned()
                .collect();
        }
        
        Vec::new()
    }
}

#[cfg(feature = "credential-audit")]
pub mod file {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;
    
    pub struct FileAuditLogger {
        file: Mutex<Option<File>>,
    }
    
    impl FileAuditLogger {
        pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            
            Ok(Self {
                file: Mutex::new(Some(file)),
            })
        }
    }
    
    impl AuditLogger for FileAuditLogger {
        fn log(&self, entry: AuditEntry) {
            if let Ok(json) = serde_json::to_string(&entry) {
                if let Ok(mut guard) = self.file.lock() {
                    if let Some(ref mut file) = *guard {
                        let _ = writeln!(file, "{}", json);
                        let _ = file.flush();
                    }
                }
            }
        }
        
        fn query(
            &self,
            _start_time: Option<DateTime<Utc>>,
            _end_time: Option<DateTime<Utc>>,
            _action: Option<AuditAction>,
            _credential_key: Option<&str>,
        ) -> Vec<AuditEntry> {
            Vec::new()
        }
    }
}

#[cfg(feature = "credential-audit")]
pub mod syslog {
    use super::*;
    use std::net::UdpSocket;
    
    pub struct SyslogAuditLogger {
        socket: UdpSocket,
        destination: String,
    }
    
    impl SyslogAuditLogger {
        pub fn new(destination: &str) -> Result<Self, std::io::Error> {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            Ok(Self {
                socket,
                destination: destination.to_string(),
            })
        }
    }
    
    impl AuditLogger for SyslogAuditLogger {
        fn log(&self, entry: AuditEntry) {
            let message = format!(
                "<{}> openagentic-credential: {} {} {} {:?}",
                14,
                entry.action,
                entry.credential_key,
                entry.result,
                entry.user_id.as_deref().unwrap_or("-")
            );
            
            let _ = self.socket.send_to(
                message.as_bytes(),
                &self.destination,
            );
        }
        
        fn query(
            &self,
            _start_time: Option<DateTime<Utc>>,
            _end_time: Option<DateTime<Utc>>,
            _action: Option<AuditAction>,
            _credential_key: Option<&str>,
        ) -> Vec<AuditEntry> {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audit_entry_creation() {
        let entry = AuditEntry::new(
            AuditAction::Read,
            "api_key".to_string(),
            AuditResult::Success,
        );
        
        assert_eq!(entry.action, AuditAction::Read);
        assert_eq!(entry.credential_key, "api_key");
        assert_eq!(entry.result, AuditResult::Success);
    }
    
    #[test]
    fn test_audit_entry_with_options() {
        let entry = AuditEntry::new(
            AuditAction::Create,
            "api_key".to_string(),
            AuditResult::Success,
        )
        .with_user("user123")
        .with_ip("192.168.1.1")
        .with_details("Created new API key");
        
        assert_eq!(entry.user_id, Some("user123".to_string()));
        assert_eq!(entry.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(entry.details, Some("Created new API key".to_string()));
    }
    
    #[test]
    fn test_in_memory_audit_logger() {
        let logger = InMemoryAuditLogger::new();
        
        logger.log(AuditEntry::new(
            AuditAction::Read,
            "api_key".to_string(),
            AuditResult::Success,
        ));
        
        let entries = logger.query(None, None, None, None);
        assert_eq!(entries.len(), 1);
    }
    
    #[test]
    fn test_audit_query_by_action() {
        let logger = InMemoryAuditLogger::new();
        
        logger.log(AuditEntry::new(AuditAction::Read, "key1".to_string(), AuditResult::Success));
        logger.log(AuditEntry::new(AuditAction::Create, "key2".to_string(), AuditResult::Success));
        logger.log(AuditEntry::new(AuditAction::Read, "key3".to_string(), AuditResult::Success));
        
        let read_entries = logger.query(None, None, Some(AuditAction::Read), None);
        assert_eq!(read_entries.len(), 2);
        
        let create_entries = logger.query(None, None, Some(AuditAction::Create), None);
        assert_eq!(create_entries.len(), 1);
    }
    
    #[test]
    fn test_audit_query_by_key() {
        let logger = InMemoryAuditLogger::new();
        
        logger.log(AuditEntry::new(AuditAction::Read, "api_key".to_string(), AuditResult::Success));
        logger.log(AuditEntry::new(AuditAction::Write, "api_key".to_string(), AuditResult::Success));
        logger.log(AuditEntry::new(AuditAction::Read, "other_key".to_string(), AuditResult::Success));
        
        let entries = logger.query(None, None, None, Some("api_key"));
        assert_eq!(entries.len(), 2);
    }
}
