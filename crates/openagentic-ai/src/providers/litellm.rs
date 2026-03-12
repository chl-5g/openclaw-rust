//! LiteLLM 统一提供商 — 通过 litellm-rs 调用 100+ LLM API
//!
//! 用一个 Provider 实现所有厂商，模型名用 "provider/model" 格式路由。

use async_trait::async_trait;
use futures::Stream;
use openagentic_core::{Message, OpenAgenticError, Role};
use std::pin::Pin;

use crate::types::{
    ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk,
    StreamDelta, TokenUsage,
};

use super::{AIProvider, ProviderConfig};

/// LiteLLM 统一提供商
pub struct LiteLLMProvider {
    config: ProviderConfig,
    /// litellm-rs 模型前缀，如 "openai/", "anthropic/", "ollama/"
    model_prefix: String,
}

impl LiteLLMProvider {
    pub fn new(config: ProviderConfig, model_prefix: impl Into<String>) -> Self {
        Self {
            config,
            model_prefix: model_prefix.into(),
        }
    }

    /// 将 openagentic Message 转为 litellm-rs ChatMessage
    fn convert_messages(messages: &[Message]) -> Vec<litellm_rs::Message> {
        messages
            .iter()
            .map(|msg| {
                let text = msg.text_content().unwrap_or("").to_string();
                match msg.role {
                    Role::System => litellm_rs::system_message(text),
                    Role::User => litellm_rs::user_message(text),
                    Role::Assistant => litellm_rs::assistant_message(text),
                    Role::Tool => {
                        // tool result → 用 user message 包一层 (litellm-rs 无原生 tool role)
                        litellm_rs::user_message(format!("[tool result] {}", text))
                    }
                }
            })
            .collect()
    }

    /// 构建 CompletionOptions
    fn build_options(&self, request: &ChatRequest) -> litellm_rs::CompletionOptions {
        let mut opts = litellm_rs::CompletionOptions {
            temperature: request.temperature,
            max_tokens: request.max_tokens.map(|v| v as u32),
            stream: request.stream,
            ..Default::default()
        };

        if let Some(ref key) = self.config.api_key {
            opts.api_key = Some(key.clone());
        }
        if let Some(ref url) = self.config.base_url {
            opts.api_base = Some(url.clone());
        }
        if let Some(ref org) = self.config.organization {
            opts.organization = Some(org.clone());
        }
        if let Some(timeout) = self.config.timeout {
            opts.timeout = Some(timeout.as_secs());
        }
        if !self.config.headers.is_empty() {
            opts.headers = Some(self.config.headers.clone());
        }

        opts
    }

    /// 获取完整模型名（加前缀）
    fn full_model(&self, model: &str) -> String {
        if model.contains('/') || self.model_prefix.is_empty() {
            model.to_string()
        } else {
            format!("{}{}", self.model_prefix, model)
        }
    }

    /// 从 litellm FinishReason 转换
    fn convert_finish_reason(
        reason: Option<litellm_rs::core::types::responses::FinishReason>,
    ) -> FinishReason {
        match reason {
            Some(litellm_rs::core::types::responses::FinishReason::Stop) => FinishReason::Stop,
            Some(litellm_rs::core::types::responses::FinishReason::Length) => FinishReason::Length,
            Some(litellm_rs::core::types::responses::FinishReason::ToolCalls) => {
                FinishReason::ToolCalls
            }
            Some(litellm_rs::core::types::responses::FinishReason::ContentFilter) => {
                FinishReason::ContentFilter
            }
            _ => FinishReason::Stop,
        }
    }
}

#[async_trait]
impl AIProvider for LiteLLMProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn chat(&self, request: ChatRequest) -> openagentic_core::Result<ChatResponse> {
        let model = self.full_model(&request.model);
        let messages = Self::convert_messages(&request.messages);
        let options = self.build_options(&request);

        let response = litellm_rs::completion(&model, messages, Some(options))
            .await
            .map_err(|e| OpenAgenticError::AIProvider(e.to_string()))?;

        let choice = response.choices.first();

        let text = choice
            .and_then(|c| c.message.content.as_ref())
            .map(|c| match c {
                litellm_rs::MessageContent::Text(t) => t.clone(),
                litellm_rs::MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|p| match p {
                        litellm_rs::ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            })
            .unwrap_or_default();

        let finish_reason =
            Self::convert_finish_reason(choice.and_then(|c| c.finish_reason.clone()));

        let usage = response
            .usage
            .as_ref()
            .map(|u| TokenUsage::new(u.prompt_tokens as usize, u.completion_tokens as usize))
            .unwrap_or_else(|| TokenUsage::new(0, 0));

        Ok(ChatResponse {
            id: response.id,
            model: response.model,
            message: Message::assistant(text),
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> openagentic_core::Result<Pin<Box<dyn Stream<Item = openagentic_core::Result<StreamChunk>> + Send>>>
    {
        let model = self.full_model(&request.model);
        let messages = Self::convert_messages(&request.messages);
        let mut options = self.build_options(&request);
        options.stream = true;

        let stream = litellm_rs::completion_stream(&model, messages, Some(options))
            .await
            .map_err(|e| OpenAgenticError::AIProvider(e.to_string()))?;

        use futures::StreamExt;
        let mapped = stream.map(|result| {
            result
                .map(|chunk| {
                    let choice = chunk.choices.first();
                    let delta_content = choice.and_then(|c| c.delta.content.clone());
                    let delta_role = choice.and_then(|c| c.delta.role.clone());
                    let finished = choice
                        .and_then(|c| c.finish_reason.as_ref())
                        .is_some();

                    StreamChunk {
                        id: chunk.id,
                        model: chunk.model,
                        delta: StreamDelta {
                            role: delta_role,
                            content: delta_content,
                            tool_calls: vec![],
                        },
                        finished,
                        finish_reason: if finished {
                            Some(FinishReason::Stop)
                        } else {
                            None
                        },
                    }
                })
                .map_err(|e| OpenAgenticError::AIProvider(e.to_string()))
        });

        Ok(Box::pin(mapped))
    }

    async fn embed(
        &self,
        request: EmbeddingRequest,
    ) -> openagentic_core::Result<EmbeddingResponse> {
        let model = self.full_model(&request.model);

        let response = litellm_rs::embedding(&model, request.input.clone(), None)
            .await
            .map_err(|e| OpenAgenticError::AIProvider(e.to_string()))?;

        let embeddings: Vec<Vec<f32>> = response.data.iter().map(|d| d.embedding.clone()).collect();

        let usage = response
            .usage
            .as_ref()
            .map(|u| TokenUsage::new(u.prompt_tokens as usize, 0))
            .unwrap_or_else(|| TokenUsage::new(0, 0));

        Ok(EmbeddingResponse {
            embeddings,
            model: response.model,
            usage,
        })
    }

    async fn models(&self) -> openagentic_core::Result<Vec<String>> {
        // 返回配置的默认模型
        Ok(vec![self.config.default_model.clone()])
    }

    async fn health_check(&self) -> openagentic_core::Result<bool> {
        // 简单测试：发一个最小请求
        let model = self.full_model(&self.config.default_model);
        let messages = vec![litellm_rs::user_message("ping")];
        let mut opts = self.build_options(&ChatRequest::new(
            &self.config.default_model,
            vec![Message::user("ping")],
        ));
        opts.max_tokens = Some(1);

        match litellm_rs::completion(&model, messages, Some(opts)).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
