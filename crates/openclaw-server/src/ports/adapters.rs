use async_trait::async_trait;
use futures::StreamExt;
use openclaw_agent::ports::{
    AIPort, CameraInfo, DevicePort, LocationInfo, MemoryEntry, MemoryPort, RecallItem, ScreenInfo,
    SecurityCheckResult, SecurityPort, ToolInfo, ToolPort,
};
use openclaw_ai::{
    AIProvider, ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, StreamChunk,
};
use openclaw_core::Result as OpenClawResult;
use openclaw_core::{Content, Message, OpenClawError, Role};
use openclaw_device::UnifiedDeviceManager;
use openclaw_memory::factory::MemoryBackend;
use openclaw_memory::MemoryManager;
use openclaw_memory::recall::{RecallItem as MemoryRecallItem, RecallResult as MemoryRecallResult};
use openclaw_memory::types::{MemoryContent, MemoryItem, MemoryRetrieval};
use openclaw_security::{SecurityPipeline, PipelineResult};
use openclaw_tools::ToolRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::RwLock;

fn memory_item_to_entry(m: MemoryItem) -> MemoryEntry {
    MemoryEntry {
        id: m.id.to_string(),
        content: m.content.to_text(),
        metadata: HashMap::new(),
    }
}

pub struct AiPortAdapter {
    pub provider: Arc<dyn AIProvider>,
}

#[async_trait]
impl AIPort for AiPortAdapter {
    async fn chat(&self, messages: Vec<Message>) -> OpenClawResult<String> {
        let request = ChatRequest::new("default", messages);
        let response: ChatResponse = self.provider.chat(request).await?;
        Ok(response
            .message
            .content
            .first()
            .and_then(|c| match c {
                Content::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default())
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> OpenClawResult<Box<dyn futures::Stream<Item = OpenClawResult<String>> + Send + Sync>> {
        let mut request = ChatRequest::new("default", messages);
        request.stream = true;

        let stream = self.provider.chat_stream(request).await?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(chunk_result) = stream.next().await {
                let content = chunk_result.map(|c| c.delta.content.unwrap_or_default());
                if tx.send(content).await.is_err() {
                    break;
                }
            }
        });

        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::new(rx)
            as Box<
                dyn futures::Stream<Item = OpenClawResult<String>> + Send + Sync,
            >)
    }

    async fn embed(&self, texts: Vec<String>) -> OpenClawResult<Vec<Vec<f32>>> {
        let request = EmbeddingRequest {
            model: "default".to_string(),
            input: texts,
        };
        let response: EmbeddingResponse = self.provider.embed(request).await?;
        Ok(response.embeddings)
    }
}

pub struct MemoryPortAdapter {
    backend: Arc<dyn MemoryBackend>,
}

impl MemoryPortAdapter {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl MemoryPort for MemoryPortAdapter {
    async fn add(&self, entry: MemoryEntry) -> OpenClawResult<()> {
        let message = Message {
            id: uuid::Uuid::new_v4(),
            role: Role::User,
            content: vec![Content::Text {
                text: entry.content,
            }],
            created_at: chrono::Utc::now(),
            metadata: Default::default(),
        };
        self.backend.add(message).await
    }

    async fn retrieve(&self, query: &str, limit: usize) -> OpenClawResult<Vec<MemoryEntry>> {
        let retrieval: MemoryRetrieval = self.backend.retrieve(query, limit).await?;
        Ok(retrieval
            .items
            .into_iter()
            .map(memory_item_to_entry)
            .collect())
    }

    async fn recall(&self, context: &str, _limit: usize) -> OpenClawResult<Vec<RecallItem>> {
        let recall_result: MemoryRecallResult = self.backend.recall(context).await?;
        Ok(recall_result
            .items
            .into_iter()
            .map(|m| RecallItem {
                entry: MemoryEntry {
                    id: m.id,
                    content: m.content,
                    metadata: HashMap::new(),
                },
                score: m.similarity,
            })
            .collect())
    }

    async fn get_context(&self) -> OpenClawResult<Vec<Message>> {
        let retrieval = self.backend.retrieve("", 10).await?;
        let messages: Vec<Message> = retrieval
            .items
            .into_iter()
            .map(|item| {
                let text = item.content.to_text();
                Message {
                    id: item.id,
                    role: Role::User,
                    content: vec![Content::Text { text }],
                    created_at: item.created_at,
                    metadata: Default::default(),
                }
            })
            .collect();
        Ok(messages)
    }
}

pub struct SecurityPortAdapter {
    pub pipeline: Arc<SecurityPipeline>,
}

#[async_trait]
impl SecurityPort for SecurityPortAdapter {
    async fn check(&self, input: &str) -> OpenClawResult<SecurityCheckResult> {
        let (result, _) = self.pipeline.check_input("security_port", input).await;

        match result {
            PipelineResult::Allow => Ok(SecurityCheckResult {
                allowed: true,
                reason: None,
            }),
            PipelineResult::Block(reason) => Ok(SecurityCheckResult {
                allowed: false,
                reason: Some(reason),
            }),
            PipelineResult::Warn(warning) => Ok(SecurityCheckResult {
                allowed: true,
                reason: Some(warning),
            }),
        }
    }
}

pub struct ToolPortAdapter {
    pub registry: Arc<ToolRegistry>,
}

impl ToolPortAdapter {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolPort for ToolPortAdapter {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> OpenClawResult<serde_json::Value> {
        if !self.registry.has_tool(tool_name) {
            return Err(OpenClawError::Tool(format!(
                "Tool '{}' not found or not available",
                tool_name
            )));
        }

        self.registry.execute(tool_name, arguments).await
    }

    async fn execute_with_sandbox(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        _enable_sandbox: bool,
    ) -> OpenClawResult<serde_json::Value> {
        self.execute(tool_name, arguments).await
    }

    async fn list_tools(&self) -> OpenClawResult<Vec<ToolInfo>> {
        let tool_names = self.registry.list_tools();
        let mut tools = Vec::new();
        for name in tool_names {
            if let Some(tool) = self.registry.get(&name) {
                tools.push(ToolInfo {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: serde_json::json!({}),
                });
            }
        }
        Ok(tools)
    }
}

pub struct DevicePortAdapter {
    manager: Arc<UnifiedDeviceManager>,
}

impl DevicePortAdapter {
    pub fn new(manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl DevicePort for DevicePortAdapter {
    async fn list_cameras(&self) -> OpenClawResult<Vec<CameraInfo>> {
        let capabilities = self.manager.list_capabilities().await;
        let cameras: Vec<CameraInfo> = capabilities
            .iter()
            .filter(|d| d.device_type.as_str() == "camera")
            .map(|c| CameraInfo {
                id: c.id.clone(),
                name: c.name.clone(),
                available: true,
            })
            .collect();
        Ok(cameras)
    }

    async fn capture_camera(&self, camera_id: &str, path: &str) -> OpenClawResult<String> {
        let result = self
            .manager
            .capture_camera(camera_id)
            .await
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        
        if result.success {
            Ok(result.data.unwrap_or_else(|| path.to_string()))
        } else {
            Err(OpenClawError::Config(result.error.unwrap_or_else(|| "Capture failed".to_string())))
        }
    }

    async fn list_screens(&self) -> OpenClawResult<Vec<ScreenInfo>> {
        let capabilities = self.manager.list_capabilities().await;
        let screens: Vec<ScreenInfo> = capabilities
            .iter()
            .filter(|d| d.device_type.as_str() == "screen")
            .map(|s| ScreenInfo {
                id: s.id.clone(),
                name: s.name.clone(),
                resolution: None,
                available: true,
            })
            .collect();
        Ok(screens)
    }

    async fn capture_screen(&self, screen_id: &str, path: &str) -> OpenClawResult<String> {
        let result = self
            .manager
            .capture_screen(screen_id)
            .await
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        
        if result.success {
            Ok(result.data.unwrap_or_else(|| path.to_string()))
        } else {
            Err(OpenClawError::Config(result.error.unwrap_or_else(|| "Capture failed".to_string())))
        }
    }

    async fn get_location(&self) -> OpenClawResult<LocationInfo> {
        Ok(LocationInfo {
            id: "default".to_string(),
            available: false,
        })
    }

    async fn start_location_tracking(&self) -> OpenClawResult<()> {
        Ok(())
    }

    async fn stop_location_tracking(&self) -> OpenClawResult<()> {
        Ok(())
    }
}
