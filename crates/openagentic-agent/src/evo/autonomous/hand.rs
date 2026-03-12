use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use super::schedule::ScheduleType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HandCategory {
    Research,
    Collection,
    Prediction,
    Media,
    Social,
    Automation,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Markdown,
    Json,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandOutputChannel {
    pub channel_type: String,
    pub channel_id: String,
    pub format: OutputFormat,
    pub on_events: Vec<String>,
}

impl Default for HandOutputChannel {
    fn default() -> Self {
        Self {
            channel_type: String::new(),
            channel_id: String::new(),
            format: OutputFormat::Markdown,
            on_events: vec!["success".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub retry_delay_secs: u64,
    pub evolve_on_failure: bool,
    pub evolve_on_success: bool,
    pub evolve_threshold: f64,
    pub enable_learning: bool,
    pub optimization_interval: u32,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_secs: 300,
            retry_delay_secs: 60,
            evolve_on_failure: false,
            evolve_on_success: false,
            evolve_threshold: 0.5,
            enable_learning: true,
            optimization_interval: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveConfig {
    pub enabled: bool,
    pub trigger_on_time: Option<String>,
    #[serde(default)]
    pub trigger_on_sequence: Vec<String>,
    pub prewarm_seconds: u32,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trigger_on_time: None,
            trigger_on_sequence: Vec::new(),
            prewarm_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum HandStatus {
    #[default]
    Active,
    Paused,
    Running,
    Failed,
    Disabled,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HandState {
    pub status: HandStatus,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    pub last_output: Option<String>,
    pub execution_count: u64,
    pub consecutive_failures: u32,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardrailAction {
    RequireApproval { prompt: String },
    Block,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guardrail {
    pub action: GuardrailAction,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub description: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCall {
    pub skill_id: String,
    pub input_template: String,
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: HandCategory,
    pub schedule: Option<ScheduleType>,
    pub system_prompt: String,
    pub skill_id: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub guardrails: Vec<Guardrail>,
    pub metrics: Vec<MetricDefinition>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub output_channels: Vec<HandOutputChannel>,
    #[serde(default)]
    pub execution_config: ExecutionConfig,
    #[serde(default)]
    pub state: HandState,
    #[serde(default)]
    pub predictive_config: Option<PredictiveConfig>,
    #[serde(default)]
    pub skill_calls: Vec<SkillCall>,
}

impl Hand {
    pub fn new(id: String, name: String, description: String, category: HandCategory) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.clone(),
            name,
            description,
            category,
            schedule: None,
            system_prompt: String::new(),
            skill_id: None,
            tools: Vec::new(),
            guardrails: Vec::new(),
            metrics: Vec::new(),
            enabled: false,
            created_at: now,
            updated_at: now,
            version: "1.0.0".to_string(),
            output_channels: Vec::new(),
            execution_config: ExecutionConfig::default(),
            state: HandState::default(),
            predictive_config: None,
            skill_calls: Vec::new(),
        }
    }

    pub fn with_schedule(mut self, schedule: ScheduleType) -> Self {
        self.schedule = Some(schedule);
        self
    }

    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }

    pub fn with_skill(mut self, skill_id: String) -> Self {
        self.skill_id = Some(skill_id);
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_guardrails(mut self, guardrails: Vec<Guardrail>) -> Self {
        self.guardrails = guardrails;
        self
    }

    pub fn with_metrics(mut self, metrics: Vec<MetricDefinition>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = chrono::Utc::now();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = chrono::Utc::now();
    }
}

pub struct HandRegistry {
    hands: Arc<RwLock<HashMap<String, Hand>>>,
}

impl HandRegistry {
    pub fn new() -> Self {
        Self {
            hands: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, hand: Hand) {
        let mut hands = self.hands.write().await;
        hands.insert(hand.id.clone(), hand);
    }

    pub async fn unregister(&self, id: &str) -> Option<Hand> {
        let mut hands = self.hands.write().await;
        hands.remove(id)
    }

    pub async fn get(&self, id: &str) -> Option<Hand> {
        let hands = self.hands.read().await;
        hands.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<Hand> {
        let hands = self.hands.read().await;
        hands.values().cloned().collect()
    }

    pub async fn list_by_category(&self, category: HandCategory) -> Vec<Hand> {
        let hands = self.hands.read().await;
        hands
            .values()
            .filter(|h| h.category == category)
            .cloned()
            .collect()
    }

    pub async fn list_enabled(&self) -> Vec<Hand> {
        let hands = self.hands.read().await;
        hands.values().filter(|h| h.enabled).cloned().collect()
    }

    pub async fn enable(&self, id: &str) -> bool {
        let mut hands = self.hands.write().await;
        if let Some(hand) = hands.get_mut(id) {
            hand.enable();
            true
        } else {
            false
        }
    }

    pub async fn disable(&self, id: &str) -> bool {
        let mut hands = self.hands.write().await;
        if let Some(hand) = hands.get_mut(id) {
            hand.disable();
            true
        } else {
            false
        }
    }

    pub async fn update(&self, hand: Hand) -> bool {
        let mut hands = self.hands.write().await;
        if hands.contains_key(&hand.id) {
            hands.insert(hand.id.clone(), hand);
            true
        } else {
            false
        }
    }

    pub async fn count(&self) -> usize {
        let hands = self.hands.read().await;
        hands.len()
    }

    pub async fn count_enabled(&self) -> usize {
        let hands = self.hands.read().await;
        hands.values().filter(|h| h.enabled).count()
    }
}

impl Default for HandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_creation() {
        let hand = Hand::new(
            "researcher".to_string(),
            "Researcher".to_string(),
            "Deep research agent".to_string(),
            HandCategory::Research,
        );

        assert_eq!(hand.id, "researcher");
        assert_eq!(hand.category, HandCategory::Research);
        assert!(!hand.enabled);
    }

    #[test]
    fn test_hand_builder() {
        let hand = Hand::new(
            "lead".to_string(),
            "Lead Generator".to_string(),
            "Generate leads".to_string(),
            HandCategory::Research,
        )
        .with_schedule(ScheduleType::Cron("0 8 * * *".to_string()))
        .with_system_prompt("You are a lead generator".to_string())
        .with_tools(vec![
            ToolDefinition {
                name: "search".to_string(),
                description: "Search the web".to_string(),
                required: true,
            }
        ]);

        assert!(hand.schedule.is_some());
        assert!(!hand.system_prompt.is_empty());
        assert_eq!(hand.tools.len(), 1);
    }

    #[tokio::test]
    async fn test_register_get() {
        let registry = HandRegistry::new();
        let hand = Hand::new(
            "test".to_string(),
            "Test".to_string(),
            "Test hand".to_string(),
            HandCategory::Custom,
        );

        registry.register(hand).await;
        let retrieved = registry.get("test").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let registry = HandRegistry::new();
        let hand = Hand::new(
            "test".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            HandCategory::Custom,
        );

        registry.register(hand).await;
        assert!(registry.enable("test").await);

        let enabled = registry.get("test").await.unwrap();
        assert!(enabled.enabled);

        assert!(registry.disable("test").await);
        let disabled = registry.get("test").await.unwrap();
        assert!(!disabled.enabled);
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let registry = HandRegistry::new();

        registry.register(Hand::new("r1".to_string(), "R1".to_string(), "".to_string(), HandCategory::Research)).await;
        registry.register(Hand::new("r2".to_string(), "R2".to_string(), "".to_string(), HandCategory::Research)).await;
        registry.register(Hand::new("c1".to_string(), "C1".to_string(), "".to_string(), HandCategory::Collection)).await;

        let research = registry.list_by_category(HandCategory::Research).await;
        assert_eq!(research.len(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let registry = HandRegistry::new();

        registry.register(Hand::new("1".to_string(), "1".to_string(), "".to_string(), HandCategory::Custom)).await;
        registry.register(Hand::new("2".to_string(), "2".to_string(), "".to_string(), HandCategory::Custom)).await;

        assert_eq!(registry.count().await, 2);

        registry.enable("1").await;
        assert_eq!(registry.count_enabled().await, 1);
    }
}
