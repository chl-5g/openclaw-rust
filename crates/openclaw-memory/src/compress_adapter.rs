use async_trait::async_trait;
use openclaw_ai::{AIProvider, ChatRequest, ChatResponse};
use openclaw_core::{Message, OpenClawError, Result, Role};
use std::sync::Arc;

use crate::compressor::AICompressProvider;

pub struct AIProviderCompressAdapter {
    provider: Arc<dyn AIProvider>,
    model: String,
}

impl AIProviderCompressAdapter {
    pub fn new(provider: Arc<dyn AIProvider>, model: String) -> Self {
        Self { provider, model }
    }
}

#[async_trait]
impl AICompressProvider for AIProviderCompressAdapter {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        if messages.is_empty() {
            return Err(OpenClawError::AIProvider("没有消息可摘要".to_string()));
        }

        let mut prompt = String::from("请为以下对话生成简洁的摘要，概括主要话题和关键信息：\n\n");
        for msg in messages {
            let role = match msg.role {
                Role::User => "用户",
                Role::Assistant => "助手",
                Role::System => "系统",
                _ => "未知",
            };
            if let Some(text) = msg.text_content() {
                prompt.push_str(&format!("{}: {}\n", role, text));
            }
        }
        prompt.push_str("\n请生成不超过100字的摘要：");

        let user_message = Message::new(Role::User, vec![openclaw_core::Content::Text {
            text: prompt,
        }]);
        
        let request = ChatRequest::new(self.model.clone(), vec![user_message])
            .with_temperature(0.3)
            .with_max_tokens(200);

        let response = self
            .provider
            .chat(request)
            .await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        Ok(response.message.text_content().unwrap_or_default().to_string())
    }
}
