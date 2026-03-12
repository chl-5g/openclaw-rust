//! Encrypted Vault Module - 加密保险库实现
//!
//! 使用 AES-256-GCM 提供端到端加密

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::rngs::OsRng;
use std::sync::Arc;

use super::vault::{Vault, VaultError};

pub struct EncryptedVault {
    cipher: Aes256Gcm,
    inner: Arc<dyn Vault>,
}

impl EncryptedVault {
    pub fn new(master_key: &[u8; 32], inner: Arc<dyn Vault>) -> Result<Self, VaultError> {
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
        
        Ok(Self { cipher, inner })
    }
    
    pub fn with_password(password: &str, inner: Arc<dyn Vault>) -> Result<Self, VaultError> {
        let key = Self::derive_key(password);
        Self::new(&key, inner)
    }
    
    fn derive_key(password: &str) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }
    
    fn generate_nonce() -> [u8; 12] {
        use rand::RngCore;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        nonce_bytes
    }
    
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
        
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, VaultError> {
        if encrypted.len() < 12 {
            return Err(VaultError::DecryptionError("Invalid encrypted data".to_string()));
        }
        
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }
}

#[async_trait]
impl Vault for EncryptedVault {
    async fn store(&self, key: &str, value: &[u8]) -> Result<(), VaultError> {
        let encrypted = self.encrypt(value)?;
        let encoded = BASE64.encode(&encrypted);
        self.inner.store(key, encoded.as_bytes()).await
    }
    
    async fn retrieve(&self, key: &str) -> Result<Vec<u8>, VaultError> {
        let encoded = self.inner.retrieve(key).await?;
        let encrypted = String::from_utf8(encoded)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))?;
        let encrypted_bytes = BASE64.decode(encrypted)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))?;
        self.decrypt(&encrypted_bytes)
    }
    
    async fn delete(&self, key: &str) -> Result<(), VaultError> {
        self.inner.delete(key).await
    }
    
    async fn exists(&self, key: &str) -> Result<bool, VaultError> {
        self.inner.exists(key).await
    }
    
    async fn list_keys(&self) -> Result<Vec<String>, VaultError> {
        self.inner.list_keys().await
    }
}

pub struct AesKeyGenerator;

impl AesKeyGenerator {
    pub fn generate() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }
    
    pub fn to_base64(key: &[u8; 32]) -> String {
        BASE64.encode(key)
    }
    
    pub fn from_base64(encoded: &str) -> Result<[u8; 32], VaultError> {
        let decoded = BASE64.decode(encoded)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
        if decoded.len() != 32 {
            return Err(VaultError::EncryptionError("Invalid key length".to_string()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        Ok(key)
    }
}

#[cfg(all(test, feature = "credential-vault"))]
mod tests {
    use super::*;
    use crate::credential::vault::memory::MemoryVault;
    
    #[tokio::test]
    async fn test_encrypted_vault_store_and_retrieve() {
        let inner = Arc::new(MemoryVault::new());
        let key = AesKeyGenerator::generate();
        let vault = EncryptedVault::new(&key, inner).unwrap();
        
        vault.store("secret-key", b"my-secret-value").await.unwrap();
        
        let result = vault.retrieve("secret-key").await.unwrap();
        assert_eq!(result, b"my-secret-value");
    }
    
    #[tokio::test]
    async fn test_encrypted_vault_different_keys() {
        let inner = Arc::new(MemoryVault::new());
        let key1 = AesKeyGenerator::generate();
        let key2 = AesKeyGenerator::generate();
        
        let vault1 = EncryptedVault::new(&key1, inner.clone()).unwrap();
        let vault2 = EncryptedVault::new(&key2, inner).unwrap();
        
        vault1.store("secret", b"value").await.unwrap();
        
        let result = vault2.retrieve("secret").await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_key_generation() {
        let key1 = AesKeyGenerator::generate();
        let key2 = AesKeyGenerator::generate();
        
        assert_ne!(key1, key2);
        
        let encoded = AesKeyGenerator::to_base64(&key1);
        let decoded = AesKeyGenerator::from_base64(&encoded).unwrap();
        
        assert_eq!(key1, decoded);
    }
}
