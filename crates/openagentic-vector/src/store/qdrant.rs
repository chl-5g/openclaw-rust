use async_trait::async_trait;
use qdrant_client::Qdrant;
use std::sync::Arc;

use crate::VectorStore;
use crate::types::{Filter, SearchQuery, SearchResult, StoreStats, VectorItem};
use openagentic_core::{OpenAgenticError, Result};

pub struct QdrantStore {
    _client: Qdrant,
    collection_name: String,
    dimension: usize,
}

impl QdrantStore {
    pub async fn new(
        url: &str,
        collection_name: &str,
        dimension: usize,
        _api_key: Option<&str>,
    ) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .map_err(|e| OpenAgenticError::Config(format!("Failed to create Qdrant client: {}", e)))?;

        Ok(Self {
            _client: client,
            collection_name: collection_name.to_string(),
            dimension,
        })
    }
}

#[async_trait]
impl VectorStore for QdrantStore {
    async fn upsert(&self, _item: VectorItem) -> Result<()> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant upsert requires full implementation".to_string(),
        ))
    }

    async fn upsert_batch(&self, _items: Vec<VectorItem>) -> Result<usize> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant upsert_batch requires full implementation".to_string(),
        ))
    }

    async fn search(&self, _query: SearchQuery) -> Result<Vec<SearchResult>> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant search requires full implementation".to_string(),
        ))
    }

    async fn get(&self, _id: &str) -> Result<Option<VectorItem>> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant get requires full implementation".to_string(),
        ))
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant delete requires full implementation".to_string(),
        ))
    }

    async fn delete_by_filter(&self, _filter: Filter) -> Result<usize> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant delete_by_filter requires full implementation".to_string(),
        ))
    }

    async fn stats(&self) -> Result<StoreStats> {
        Ok(StoreStats {
            total_vectors: 0,
            total_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn clear(&self) -> Result<()> {
        Err(OpenAgenticError::VectorStore(
            "Qdrant clear requires full implementation".to_string(),
        ))
    }
}

#[cfg(feature = "qdrant")]
pub struct QdrantStoreFactory;

#[cfg(feature = "qdrant")]
#[async_trait]
impl super::factory::VectorStoreFactory for QdrantStoreFactory {
    fn name(&self) -> &str {
        "qdrant"
    }

    async fn create(&self, config: &super::factory::BackendConfig) -> Result<Arc<dyn super::VectorStore>> {
        let url = config
            .url
            .as_ref()
            .ok_or_else(|| OpenAgenticError::Config("Qdrant requires url config".to_string()))?;
        
        let collection = config
            .collection
            .clone()
            .unwrap_or_else(|| "openagentic_vectors".to_string());
        
        let dimension = config.dimensions.unwrap_or(1536);
        
        let store = QdrantStore::new(
            url,
            &collection,
            dimension,
            config.api_key.as_deref(),
        ).await?;
        
        Ok(Arc::new(store) as Arc<dyn super::VectorStore>)
    }
}

#[cfg(feature = "qdrant")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::factory::VectorStoreFactory;

    #[test]
    fn test_qdrant_factory_name() {
        let factory = QdrantStoreFactory;
        assert_eq!(factory.name(), "qdrant");
    }

    #[test]
    fn test_qdrant_factory_supports_backend() {
        let factory = QdrantStoreFactory;
        assert!(factory.supports_backend("qdrant"));
        assert!(!factory.supports_backend("memory"));
    }
}
