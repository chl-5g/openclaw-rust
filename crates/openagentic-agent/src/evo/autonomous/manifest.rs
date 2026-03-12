use serde::{Deserialize, Serialize};

use super::hand::{
    ExecutionConfig, Guardrail, GuardrailAction, Hand, HandCategory, HandOutputChannel, HandState,
    OutputFormat, ScheduleType, ToolDefinition,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSettings {
    pub enabled: bool,
    pub schedule: Option<String>,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub retry_delay_secs: u64,
}

impl Default for ManifestSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            schedule: None,
            max_retries: 3,
            timeout_secs: 300,
            retry_delay_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestGuardrail {
    pub action: String,
    pub description: String,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestOutput {
    pub channel: String,
    pub target: String,
    pub format: Option<String>,
    pub on: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct HandBuilder {
    name: String,
    version: String,
    description: String,
    author: Option<String>,
    tags: Vec<String>,
    settings: ManifestSettings,
    tools: Vec<String>,
    guardrails: Vec<ManifestGuardrail>,
    output: Vec<ManifestOutput>,
}

impl HandBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            author: None,
            tags: Vec::new(),
            settings: ManifestSettings::default(),
            tools: Vec::new(),
            guardrails: Vec::new(),
            output: Vec::new(),
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.settings.enabled = enabled;
        self
    }

    pub fn schedule(mut self, schedule: impl Into<String>) -> Self {
        self.settings.schedule = Some(schedule.into());
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.settings.max_retries = retries;
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.settings.timeout_secs = secs;
        self
    }

    pub fn tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    pub fn guardrail(mut self, action: &str, description: &str, prompt: Option<String>) -> Self {
        self.guardrails.push(ManifestGuardrail {
            action: action.to_string(),
            description: description.to_string(),
            prompt,
        });
        self
    }

    pub fn output_channel(
        mut self,
        channel: &str,
        target: &str,
        format: Option<&str>,
        on: Option<Vec<String>>,
    ) -> Self {
        self.output.push(ManifestOutput {
            channel: channel.to_string(),
            target: target.to_string(),
            format: format.map(|s| s.to_string()),
            on,
        });
        self
    }

    pub fn build(self) -> Hand {
        let now = chrono::Utc::now();

        let tools: Vec<ToolDefinition> = self
            .tools
            .iter()
            .map(|t| ToolDefinition {
                name: t.clone(),
                description: String::new(),
                required: true,
            })
            .collect();

        let guardrails: Vec<Guardrail> = self
            .guardrails
            .iter()
            .map(|g| {
                let action = match g.action.as_str() {
                    "block" => GuardrailAction::Block,
                    "log" => GuardrailAction::Log,
                    _ => GuardrailAction::RequireApproval {
                        prompt: g.prompt.clone().unwrap_or_else(|| "Approval required".to_string()),
                    },
                };
                Guardrail {
                    action,
                    description: g.description.clone(),
                }
            })
            .collect();

        let output_channels: Vec<HandOutputChannel> = self
            .output
            .iter()
            .map(|o| HandOutputChannel {
                channel_type: o.channel.clone(),
                channel_id: o.target.clone(),
                format: match o.format.as_deref() {
                    Some("json") => OutputFormat::Json,
                    Some("text") => OutputFormat::Text,
                    _ => OutputFormat::Markdown,
                },
                on_events: o.on.clone().unwrap_or_else(|| vec!["success".to_string()]),
            })
            .collect();

        let schedule = self.settings.schedule.as_ref().map(|s| ScheduleType::Cron(s.clone()));

        Hand {
            id: self.name.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            category: HandCategory::Custom,
            schedule,
            system_prompt: String::new(),
            skill_id: None,
            tools,
            guardrails,
            metrics: vec![],
            enabled: self.settings.enabled,
            created_at: now,
            updated_at: now,
            version: self.version,
            output_channels,
            execution_config: ExecutionConfig {
                max_retries: self.settings.max_retries,
                timeout_secs: self.settings.timeout_secs,
                retry_delay_secs: self.settings.retry_delay_secs,
                evolve_on_failure: false,
                evolve_on_success: false,
                evolve_threshold: 0.5,
                enable_learning: true,
                optimization_interval: 10,
            },
            state: HandState::default(),
            predictive_config: None,
            skill_calls: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_builder_minimal() {
        let hand = HandBuilder::new("test", "1.0.0", "A test hand").build();
        
        assert_eq!(hand.id, "test");
        assert_eq!(hand.version, "1.0.0");
        assert!(hand.enabled);
    }

    #[test]
    fn test_hand_builder_full() {
        let hand = HandBuilder::new("researcher", "1.0.0", "Research hand")
            .tags(vec!["research".to_string(), "ai".to_string()])
            .enabled(true)
            .schedule("0 6 * * *")
            .max_retries(3)
            .timeout(600)
            .tools(vec!["web_search".to_string(), "web_fetch".to_string()])
            .guardrail("require_approval", "Check content", Some("Is this safe?".to_string()))
            .output_channel("telegram", "chat_123", Some("markdown"), Some(vec!["success".to_string()]))
            .build();
        
        assert_eq!(hand.id, "researcher");
        assert!(hand.schedule.is_some());
        assert_eq!(hand.tools.len(), 2);
        assert_eq!(hand.guardrails.len(), 1);
        assert_eq!(hand.output_channels.len(), 1);
    }

    #[test]
    fn test_hand_builder_defaults() {
        let hand = HandBuilder::new("test", "1.0", "Test").build();
        
        assert!(hand.enabled);
        assert_eq!(hand.execution_config.max_retries, 3);
        assert_eq!(hand.execution_config.timeout_secs, 300);
    }
}
