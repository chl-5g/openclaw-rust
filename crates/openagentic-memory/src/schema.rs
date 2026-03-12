//! Memory Schema - 统一的向量存储 payload schema 定义
//!
//! 定义 memory 模块中与向量存储交互时的统一字段 key

pub const CONTENT: &str = "content";
pub const TEXT_PREVIEW: &str = "text_preview";
pub const EMBEDDING: &str = "embedding";
pub const TIMESTAMP: &str = "timestamp";
pub const MEMORY_LEVEL: &str = "memory_level";
pub const MEMORY_ID: &str = "memory_id";
pub const CATEGORY: &str = "category";
pub const SOURCE: &str = "source";
pub const IMPORTANCE: &str = "importance";
pub const TAGS: &str = "tags";
pub const METADATA: &str = "metadata";
