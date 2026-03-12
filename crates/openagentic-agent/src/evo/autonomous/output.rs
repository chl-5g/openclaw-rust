use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

use super::hand::{Hand, OutputFormat};
use crate::channels::ChannelManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub hand_id: String,
    pub success: bool,
    pub output: HashMap<String, String>,
    pub tool_calls: Option<Vec<String>>,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            execution_id: String::new(),
            hand_id: String::new(),
            success: false,
            output: HashMap::new(),
            tool_calls: None,
            duration_ms: 0,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTemplate {
    pub name: String,
    pub format: String,
    pub template: String,
}

pub struct HandOutputManager {
    templates: Arc<RwLock<HashMap<String, OutputTemplate>>>,
    channel_manager: Option<Arc<ChannelManager>>,
}

impl HandOutputManager {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
            channel_manager: None,
        }
    }

    pub fn with_channel_manager(mut self, manager: Arc<ChannelManager>) -> Self {
        self.channel_manager = Some(manager);
        self
    }

    pub async fn send_to_channels(&self, hand: &Hand, result: &ExecutionResult) -> Result<(), String> {
        if let Some(ref manager) = self.channel_manager {
            let messages = self.format_result(hand, result).await;
            
            for (channel_type, message) in messages {
                let send_msg = openagentic_channels::SendMessage {
                    chat_id: channel_type.clone(),
                    message_type: "text".to_string(),
                    content: message,
                    title: Some(hand.name.clone()),
                    url: None,
                    at_mobiles: None,
                    mentioned_list: None,
                    base64: None,
                    md5: None,
                    articles: None,
                    media_id: None,
                };
                
                if let Err(e) = manager.send_to_channel(&channel_type, send_msg).await {
                    tracing::error!("Failed to send to channel {}: {}", channel_type, e);
                }
            }
        }
        Ok(())
    }

    pub async fn format_result(&self, hand: &Hand, result: &ExecutionResult) -> Vec<(String, String)> {
        let mut messages = Vec::new();
        
        for output_config in &hand.output_channels {
            if !self.should_send(&output_config.on_events, result.success) {
                continue;
            }

            let message = self.format_message(&output_config.format, hand, result);
            messages.push((output_config.channel_type.clone(), message));
        }
        
        messages
    }

    fn should_send(&self, events: &[String], success: bool) -> bool {
        events.iter().any(|e| match e.as_str() {
            "success" => success,
            "failure" => !success,
            "always" => true,
            _ => false,
        })
    }

    fn format_message(&self, format: &OutputFormat, hand: &Hand, result: &ExecutionResult) -> String {
        match format {
            OutputFormat::Markdown => self.to_markdown(hand, result),
            OutputFormat::Json => serde_json::to_string_pretty(result).unwrap_or_default(),
            OutputFormat::Text => self.to_text(hand, result),
        }
    }

    fn to_markdown(&self, hand: &Hand, result: &ExecutionResult) -> String {
        let status = if result.success { "✅ 成功" } else { "❌ 失败" };
        
        let mut output_text = String::new();
        for (key, value) in &result.output {
            output_text.push_str(&format!("- **{}**: {}\n", key, value));
        }

        format!(
            "# 🤖 {} 执行结果\n\n\
            **状态**: {}\n\
            **Hand**: {}\n\
            **时间**: {}\n\
            **耗时**: {}ms\n\n\
            ## 输出\n\n\
            {}",
            hand.name,
            status,
            hand.name,
            result.timestamp.format("%Y-%m-%d %H:%M:%S"),
            result.duration_ms,
            output_text
        )
    }

    fn to_text(&self, hand: &Hand, result: &ExecutionResult) -> String {
        let status = if result.success { "SUCCESS" } else { "FAILED" };
        
        let mut output_text = String::new();
        for (key, value) in &result.output {
            output_text.push_str(&format!("{}: {}\n", key, value));
        }

        format!(
            "[{}] {} - {}ms\n\n{}",
            status,
            hand.name,
            result.duration_ms,
            output_text
        )
    }

    pub async fn register_template(&self, template: OutputTemplate) {
        let mut templates = self.templates.write().await;
        templates.insert(template.name.clone(), template);
    }

    pub async fn apply_template(&self, template_name: &str, hand: &Hand, result: &ExecutionResult) -> Option<String> {
        let templates = self.templates.read().await;
        let template = templates.get(template_name)?;
        
        let mut content = template.template.clone();
        content = content.replace("{hand_name}", &hand.name);
        content = content.replace("{hand_id}", &hand.id);
        content = content.replace("{execution_id}", &result.execution_id);
        content = content.replace("{status}", if result.success { "success" } else { "failure" });
        content = content.replace("{duration_ms}", &result.duration_ms.to_string());
        content = content.replace("{timestamp}", &result.timestamp.to_rfc3339());
        
        Some(content)
    }
}

impl Default for HandOutputManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_send_success_event() {
        let manager = HandOutputManager::default();
        
        assert!(manager.should_send(&vec!["success".to_string()], true));
        assert!(!manager.should_send(&vec!["success".to_string()], false));
    }

    #[test]
    fn test_should_send_failure_event() {
        let manager = HandOutputManager::default();
        
        assert!(!manager.should_send(&vec!["failure".to_string()], true));
        assert!(manager.should_send(&vec!["failure".to_string()], false));
    }

    #[test]
    fn test_should_send_always_event() {
        let manager = HandOutputManager::default();
        
        assert!(manager.should_send(&vec!["always".to_string()], true));
        assert!(manager.should_send(&vec!["always".to_string()], false));
    }

    #[tokio::test]
    async fn test_output_template_replace() {
        let manager = HandOutputManager::new();
        let hand = Hand::new(
            "test_hand".to_string(),
            "Test Hand".to_string(),
            "A test hand".to_string(),
            super::super::hand::HandCategory::Custom,
        );
        
        let result = ExecutionResult {
            execution_id: "exec_123".to_string(),
            hand_id: "test_hand".to_string(),
            success: true,
            output: HashMap::new(),
            tool_calls: None,
            duration_ms: 100,
            timestamp: chrono::Utc::now(),
        };

        let template = OutputTemplate {
            name: "test".to_string(),
            format: "markdown".to_string(),
            template: "Hand: {hand_name}, Status: {status}".to_string(),
        };

        manager.register_template(template).await;
        let applied = manager.apply_template("test", &hand, &result).await;
        assert!(applied.is_some());
        let content = applied.unwrap();
        assert!(content.contains("Hand: Test Hand"));
        assert!(content.contains("Status: success"));
    }

    #[test]
    fn test_execution_result_default() {
        let result = ExecutionResult::default();
        assert!(result.execution_id.is_empty());
        assert!(!result.success);
        assert!(result.output.is_empty());
    }

    #[test]
    fn test_to_markdown_format() {
        let manager = HandOutputManager::default();
        let hand = Hand::new(
            "test_hand".to_string(),
            "Test Hand".to_string(),
            "A test hand".to_string(),
            super::super::hand::HandCategory::Custom,
        );
        
        let result = ExecutionResult {
            execution_id: "exec_123".to_string(),
            hand_id: "test_hand".to_string(),
            success: true,
            output: HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ]),
            tool_calls: None,
            duration_ms: 150,
            timestamp: chrono::Utc::now(),
        };

        let markdown = manager.to_markdown(&hand, &result);
        assert!(markdown.contains("✅ 成功"));
        assert!(markdown.contains("Test Hand"));
        assert!(markdown.contains("key1"));
        assert!(markdown.contains("value1"));
    }

    #[test]
    fn test_to_text_format() {
        let manager = HandOutputManager::default();
        let hand = Hand::new(
            "test_hand".to_string(),
            "Test Hand".to_string(),
            "A test hand".to_string(),
            super::super::hand::HandCategory::Custom,
        );
        
        let result = ExecutionResult {
            execution_id: "exec_123".to_string(),
            hand_id: "test_hand".to_string(),
            success: false,
            output: HashMap::from([
                ("error".to_string(), "something went wrong".to_string()),
            ]),
            tool_calls: None,
            duration_ms: 200,
            timestamp: chrono::Utc::now(),
        };

        let text = manager.to_text(&hand, &result);
        assert!(text.contains("FAILED"));
        assert!(text.contains("error: something went wrong"));
    }

    #[tokio::test]
    async fn test_format_result_with_channels() {
        let manager = HandOutputManager::new();
        let mut hand = Hand::new(
            "test_hand".to_string(),
            "Test Hand".to_string(),
            "A test hand".to_string(),
            super::super::hand::HandCategory::Custom,
        );
        
        hand.output_channels = vec![
            super::super::hand::HandOutputChannel {
                channel_type: "telegram".to_string(),
                channel_id: "chat_123".to_string(),
                format: OutputFormat::Markdown,
                on_events: vec!["success".to_string()],
            },
        ];

        let result = ExecutionResult {
            execution_id: "exec_123".to_string(),
            hand_id: "test_hand".to_string(),
            success: true,
            output: HashMap::new(),
            tool_calls: None,
            duration_ms: 100,
            timestamp: chrono::Utc::now(),
        };

        let messages = manager.format_result(&hand, &result).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "telegram");
    }

    #[tokio::test]
    async fn test_format_result_skips_failure() {
        let manager = HandOutputManager::new();
        let mut hand = Hand::new(
            "test_hand".to_string(),
            "Test Hand".to_string(),
            "A test hand".to_string(),
            super::super::hand::HandCategory::Custom,
        );
        
        hand.output_channels = vec![
            super::super::hand::HandOutputChannel {
                channel_type: "telegram".to_string(),
                channel_id: "chat_123".to_string(),
                format: OutputFormat::Markdown,
                on_events: vec!["success".to_string()],
            },
        ];

        let result = ExecutionResult {
            execution_id: "exec_123".to_string(),
            hand_id: "test_hand".to_string(),
            success: false,
            output: HashMap::new(),
            tool_calls: None,
            duration_ms: 100,
            timestamp: chrono::Utc::now(),
        };

        let messages = manager.format_result(&hand, &result).await;
        assert!(messages.is_empty());
    }
}
