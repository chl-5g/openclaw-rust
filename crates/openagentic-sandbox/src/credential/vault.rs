//! Vault Module - 加密保险库
//!
//! 提供安全的凭证存储抽象，支持多种后端

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Vault operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[async_trait]
pub trait Vault: Send + Sync {
    async fn store(&self, key: &str, value: &[u8]) -> Result<(), VaultError>;
    
    async fn retrieve(&self, key: &str) -> Result<Vec<u8>, VaultError>;
    
    async fn delete(&self, key: &str) -> Result<(), VaultError>;
    
    async fn exists(&self, key: &str) -> Result<bool, VaultError>;
    
    async fn list_keys(&self) -> Result<Vec<String>, VaultError>;
}

#[cfg(feature = "credential-vault")]
pub mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    pub struct MemoryVault {
        store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    }
    
    impl Default for MemoryVault {
        fn default() -> Self {
            Self::new()
        }
    }
    
    impl MemoryVault {
        pub fn new() -> Self {
            Self {
                store: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }
    
    #[async_trait]
    impl Vault for MemoryVault {
        async fn store(&self, key: &str, value: &[u8]) -> Result<(), VaultError> {
            let mut store = self.store.write().await;
            store.insert(key.to_string(), value.to_vec());
            Ok(())
        }
        
        async fn retrieve(&self, key: &str) -> Result<Vec<u8>, VaultError> {
            let store = self.store.read().await;
            store.get(key)
                .cloned()
                .ok_or_else(|| VaultError::KeyNotFound(key.to_string()))
        }
        
        async fn delete(&self, key: &str) -> Result<(), VaultError> {
            let mut store = self.store.write().await;
            store.remove(key);
            Ok(())
        }
        
        async fn exists(&self, key: &str) -> Result<bool, VaultError> {
            let store = self.store.read().await;
            Ok(store.contains_key(key))
        }
        
        async fn list_keys(&self) -> Result<Vec<String>, VaultError> {
            let store = self.store.read().await;
            Ok(store.keys().cloned().collect())
        }
    }
    
    #[cfg(test)]
    mod tests {
        use super::*;
        
        #[tokio::test]
        async fn test_store_and_retrieve() {
            let vault = MemoryVault::new();
            
            vault.store("key1", b"value1").await.unwrap();
            
            let result = vault.retrieve("key1").await.unwrap();
            assert_eq!(result, b"value1");
        }
        
        #[tokio::test]
        async fn test_delete() {
            let vault = MemoryVault::new();
            
            vault.store("key1", b"value1").await.unwrap();
            vault.delete("key1").await.unwrap();
            
            let result = vault.exists("key1").await.unwrap();
            assert!(!result);
        }
        
        #[tokio::test]
        async fn test_list_keys() {
            let vault = MemoryVault::new();
            
            vault.store("key1", b"value1").await.unwrap();
            vault.store("key2", b"value2").await.unwrap();
            
            let keys = vault.list_keys().await.unwrap();
            assert_eq!(keys.len(), 2);
        }
        
        #[tokio::test]
        async fn test_key_not_found() {
            let vault = MemoryVault::new();
            
            let result = vault.retrieve("nonexistent").await;
            assert!(result.is_err());
        }
    }
}

#[cfg(feature = "credential-sqlite")]
pub mod sqlite {
    use super::*;
    use rusqlite::{params, Connection};
    use std::path::PathBuf;
    use std::sync::Mutex;
    
    pub struct SqliteVault {
        conn: Mutex<Connection>,
    }
    
    impl SqliteVault {
        pub fn new(path: PathBuf) -> Result<Self, VaultError> {
            let conn = Connection::open(&path)
                .map_err(|e| VaultError::StorageError(e.to_string()))?;
            
            conn.execute(
                "CREATE TABLE IF NOT EXISTS vault (
                    key TEXT PRIMARY KEY,
                    value BLOB NOT NULL
                )",
                [],
            ).map_err(|e| VaultError::StorageError(e.to_string()))?;
            
            Ok(Self { conn: Mutex::new(conn) })
        }
        
        pub fn new_in_memory() -> Result<Self, VaultError> {
            let conn = Connection::open_in_memory()
                .map_err(|e| VaultError::StorageError(e.to_string()))?;
            
            conn.execute(
                "CREATE TABLE vault (
                    key TEXT PRIMARY KEY,
                    value BLOB NOT NULL
                )",
                [],
            ).map_err(|e| VaultError::StorageError(e.to_string()))?;
            
            Ok(Self { conn: Mutex::new(conn) })
        }
    }
    
    #[async_trait]
    impl Vault for SqliteVault {
        async fn store(&self, key: &str, value: &[u8]) -> Result<(), VaultError> {
            let conn = self.conn.lock().map_err(|e| VaultError::StorageError(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO vault (key, value) VALUES (?1, ?2)",
                params![key, value],
            ).map_err(|e| VaultError::StorageError(e.to_string()))?;
            Ok(())
        }
        
        async fn retrieve(&self, key: &str) -> Result<Vec<u8>, VaultError> {
            let conn = self.conn.lock().map_err(|e| VaultError::StorageError(e.to_string()))?;
            let result: Result<Vec<u8>, _> = conn.query_row(
                "SELECT value FROM vault WHERE key = ?1",
                params![key],
                |row| row.get(0),
            );
            
            match result {
                Ok(value) => Ok(value),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    Err(VaultError::KeyNotFound(key.to_string()))
                }
                Err(e) => Err(VaultError::StorageError(e.to_string())),
            }
        }
        
        async fn delete(&self, key: &str) -> Result<(), VaultError> {
            let conn = self.conn.lock().map_err(|e| VaultError::StorageError(e.to_string()))?;
            conn.execute("DELETE FROM vault WHERE key = ?1", params![key])
                .map_err(|e| VaultError::StorageError(e.to_string()))?;
            Ok(())
        }
        
        async fn exists(&self, key: &str) -> Result<bool, VaultError> {
            let conn = self.conn.lock().map_err(|e| VaultError::StorageError(e.to_string()))?;
            let count: i32 = conn.query_row(
                "SELECT COUNT(*) FROM vault WHERE key = ?1",
                params![key],
                |row| row.get(0),
            ).map_err(|e| VaultError::StorageError(e.to_string()))?;
            Ok(count > 0)
        }
        
        async fn list_keys(&self) -> Result<Vec<String>, VaultError> {
            let conn = self.conn.lock().map_err(|e| VaultError::StorageError(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT key FROM vault")
                .map_err(|e| VaultError::StorageError(e.to_string()))?;
            
            let keys = stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| VaultError::StorageError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            
            Ok(keys)
        }
    }
}
