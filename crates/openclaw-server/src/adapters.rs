use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use openclaw_agent::ports::{
    AIPort, MemoryPort, SecurityCheckResult, SecurityPort,
    ToolInfo, ToolPort,
};
use openclaw_ai::{
    AIProvider,
    types::{ChatRequest, StreamChunk},
};
use openclaw_core::{Content, Message, Result as OpenClawResult};
use openclaw_memory::MemoryManager;
use openclaw_sandbox::{SandboxManager, ToolSandboxConfig};
use openclaw_security::SecurityPipeline;
use openclaw_tools::ToolRegistry;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AIProviderAdapter {
    provider: Arc<dyn AIProvider>,
    model: String,
}

impl AIProviderAdapter {
    pub fn new(provider: Arc<dyn AIProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }
}

#[async_trait]
impl AIPort for AIProviderAdapter {
    async fn chat(&self, messages: Vec<Message>) -> OpenClawResult<String> {
        let request = ChatRequest::new(self.model.clone(), messages);

        let response = self.provider.chat(request).await?;
        Ok(response
            .message
            .content
            .first()
            .map(|c| match c {
                Content::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default())
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> OpenClawResult<Box<dyn futures::Stream<Item = OpenClawResult<String>> + Send + Sync>> {
        let mut request = ChatRequest::new(self.model.clone(), messages);
        request.stream = true;

        let stream = self.provider.chat_stream(request).await?;

        let (tx, rx) = mpsc::channel(100);

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
        use openclaw_ai::types::EmbeddingRequest;

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts,
        };

        let response = self.provider.embed(request).await?;
        Ok(response.embeddings)
    }
}

pub struct SecurityPipelineAdapter {
    pipeline: Arc<SecurityPipeline>,
}

impl SecurityPipelineAdapter {
    pub fn new(pipeline: Arc<SecurityPipeline>) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl SecurityPort for SecurityPipelineAdapter {
    async fn check(&self, input: &str) -> OpenClawResult<SecurityCheckResult> {
        let (result, _) = self.pipeline.check_input("default", input).await;

        match result {
            openclaw_security::PipelineResult::Allow => Ok(SecurityCheckResult {
                allowed: true,
                reason: None,
            }),
            openclaw_security::PipelineResult::Block(reason) => Ok(SecurityCheckResult {
                allowed: false,
                reason: Some(reason),
            }),
            openclaw_security::PipelineResult::Warn(reason) => Ok(SecurityCheckResult {
                allowed: true,
                reason: Some(reason),
            }),
        }
    }
}

pub struct ToolRegistryAdapter {
    registry: Arc<ToolRegistry>,
    sandbox_manager: Option<Arc<SandboxManager>>,
}

impl ToolRegistryAdapter {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            sandbox_manager: None,
        }
    }

    pub fn with_sandbox(mut self, sandbox_manager: Arc<SandboxManager>) -> Self {
        self.sandbox_manager = Some(sandbox_manager);
        self
    }
}

#[async_trait]
impl ToolPort for ToolRegistryAdapter {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> OpenClawResult<serde_json::Value> {
        self.registry.execute(tool_name, arguments).await
    }

    async fn execute_with_sandbox(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        enable_sandbox: bool,
    ) -> OpenClawResult<serde_json::Value> {
        if !enable_sandbox {
            return self.registry.execute(tool_name, arguments).await;
        }

        let Some(ref sandbox) = self.sandbox_manager else {
            return self.registry.execute(tool_name, arguments).await;
        };

        let registry = Arc::new(self.registry.clone());
        let tool_name = tool_name.to_string();
        let input = arguments;

        let result = sandbox
            .execute_with_security(
                &tool_name,
                input,
                None,
                {
                    let registry = registry.clone();
                    let tool_name = tool_name.clone();
                    move |args, _caps| {
                        let registry = registry.clone();
                        let tool_name = tool_name.clone();
                        async move {
                            registry
                                .execute(&tool_name, args)
                                .await
                                .map_err(|e| e.to_string())
                        }
                    }
                },
            )
            .await
            .map_err(|e| openclaw_core::OpenClawError::Tool(e.to_string()))?;

        let output = serde_json::from_str(&result.stdout)
            .map_err(|e| openclaw_core::OpenClawError::Tool(format!("Failed to parse output: {}", e)))?;
        Ok(output)
    }

    async fn list_tools(&self) -> OpenClawResult<Vec<ToolInfo>> {
        let tools = self.registry.list_tools();

        Ok(tools
            .into_iter()
            .map(|name| ToolInfo {
                name: name.clone(),
                description: String::new(),
                parameters: serde_json::json!({}),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_pipeline_adapter_check() {
        let pipeline = Arc::new(SecurityPipeline::default());
        let adapter = SecurityPipelineAdapter::new(pipeline);

        let result = adapter.check("hello").await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }

    #[tokio::test]
    async fn test_tool_registry_adapter_execute() {
        let registry = Arc::new(ToolRegistry::new());
        let adapter = ToolRegistryAdapter::new(registry);

        let result = adapter
            .execute("mock_tool", serde_json::json!({"key": "value"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_registry_adapter_list_tools() {
        let registry = Arc::new(ToolRegistry::new());
        let adapter = ToolRegistryAdapter::new(registry);

        let result = adapter.list_tools().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
