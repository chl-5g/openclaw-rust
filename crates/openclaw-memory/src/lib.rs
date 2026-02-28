//! OpenClaw Memory - 分层记忆系统
//!
//! 实现三层记忆架构：
//! - 工作记忆 (Working Memory): 最近消息，高优先级
//! - 短期记忆 (Short-term Memory): 压缩摘要，中优先级
//! - 长期记忆 (Long-term Memory): 向量存储，低优先级

pub mod factory;
pub mod ai_adapter;
pub mod bm25;
pub mod checkpoint;
pub mod checkpoint_store;
pub mod chunk;
pub mod compressor;
pub mod compress_adapter;
pub mod config;
pub mod conflict_resolver;
pub mod embedding;
pub mod fact_extractor;
pub mod file_tracker;
pub mod file_watcher;
pub mod graph_context;
pub mod hybrid_search;
pub mod knowledge_graph;
pub mod maintenance_scheduler;
pub mod manager;
pub mod pruning;
pub mod recall;
pub mod recall_strategy;
pub mod scorer;
pub mod store;
pub mod schema;
pub mod traits;
pub mod types;
pub mod unified_search;

pub mod working;
pub mod workspace;
pub mod workspace_config;

pub use config::{create_memory_store, create_memory_store_from_config, create_embedding_provider_from_config, MemoryBackend as MemoryStoreBackend};
pub use factory::{MemoryBackend, HybridMemoryBackend, MemoryManagerFactory, HybridMemoryFactory};
pub use manager::MemoryManager;
pub use types::{MemoryConfig, MemoryContent, MemoryItem, MemoryLevel, MemoryRetrieval};
pub use recall::{MemoryRecall, RecallResult, RecallConfig, SimpleMemoryRecall};
pub use scorer::ImportanceScorer;
pub use compressor::MemoryCompressor;
pub use working::WorkingMemory;
pub use workspace_config::{WorkspaceConfig, WorkspacesConfig};

pub use bm25::Bm25Index;
pub use chunk::ChunkManager;
pub use file_tracker::{FileTracker, FileTrackerConfig};
pub use recall_strategy::{RecallStrategy, RecallItem};
pub use workspace::AgentWorkspace;
pub use schema::{CONTENT, TEXT_PREVIEW, EMBEDDING, TIMESTAMP, MEMORY_LEVEL, MEMORY_ID, CATEGORY, SOURCE, IMPORTANCE, TAGS, METADATA};
