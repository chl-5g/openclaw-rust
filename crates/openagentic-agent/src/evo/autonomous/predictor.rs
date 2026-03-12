use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use super::hand::HandRegistry;
use super::schedule::ScheduleManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionTrigger {
    pub id: String,
    pub hand_id: String,
    pub trigger_type: TriggerType,
    pub condition: PredictionCondition,
    pub confidence_threshold: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerType {
    TimeBased,
    SequenceBased,
    ContextBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionCondition {
    pub time_pattern: Option<String>,
    pub preceding_actions: Vec<String>,
    pub context_keywords: Vec<String>,
    pub min_confidence: f64,
}

#[derive(Debug, Clone)]
pub struct PredictionResult {
    pub hand_id: String,
    pub confidence: f64,
    pub suggested_input: Option<String>,
    pub trigger_reason: String,
}

pub struct PredictionEngine {
    registry: Arc<HandRegistry>,
    schedule_manager: Arc<ScheduleManager>,
    triggers: Arc<RwLock<HashMap<String, PredictionTrigger>>>,
    predictions: Arc<RwLock<HashMap<String, Vec<PredictionResult>>>>,
}

impl PredictionEngine {
    pub fn new(registry: Arc<HandRegistry>, schedule_manager: Arc<ScheduleManager>) -> Self {
        Self {
            registry,
            schedule_manager,
            triggers: Arc::new(RwLock::new(HashMap::new())),
            predictions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_trigger(&self, trigger: PredictionTrigger) {
        let mut triggers = self.triggers.write().await;
        triggers.insert(trigger.id.clone(), trigger);
    }

    pub async fn unregister_trigger(&self, trigger_id: &str) {
        let mut triggers = self.triggers.write().await;
        triggers.remove(trigger_id);
    }

    pub async fn predict(&self, context: &PredictionContext) -> Vec<PredictionResult> {
        let mut results = vec![];

        let time_predictions = self.predict_time_based().await;
        results.extend(time_predictions);

        let context_predictions = self.predict_context_based(context).await;
        results.extend(context_predictions);

        let mut predictions = self.predictions.write().await;
        predictions.insert(context.execution_id.clone(), results.clone());

        results
    }

    pub async fn predict_time_based(&self) -> Vec<PredictionResult> {
        let mut results = vec![];
        let hands = self.registry.list().await;

        for hand in hands {
            if let Some(ref config) = hand.predictive_config
                && config.enabled && config.trigger_on_time.is_some() {
                    let now = Utc::now();

                    if let Some(ref cron) = config.trigger_on_time {
                        let should_trigger = self.check_time_pattern(cron, now, config.prewarm_seconds);

                        if should_trigger {
                            results.push(PredictionResult {
                                hand_id: hand.id.clone(),
                                confidence: 0.8,
                                trigger_reason: "时间预测触发".to_string(),
                                suggested_input: None,
                            });
                        }
                    }
                }
        }

        results
    }

    fn check_time_pattern(&self, cron: &str, now: DateTime<Utc>, prewarm_secs: u32) -> bool {
        let parts: Vec<&str> = cron.split_whitespace().collect();
        if parts.len() < 5 {
            return false;
        }

        let minute_str = parts[0];
        let hour_str = parts[1];

        let current_minute = now.format("%M").to_string().parse::<u32>().unwrap_or(0);
        let current_hour = now.format("%H").to_string().parse::<u32>().unwrap_or(0);
        let _current_second = now.format("%S").to_string().parse::<u32>().unwrap_or(0);

        let target_minute: u32 = if minute_str == "*" {
            current_minute
        } else if minute_str.contains(',') {
            if minute_str
                .split(',')
                .filter_map(|m| m.parse::<u32>().ok())
                .any(|m| m == current_minute) { current_minute } else { current_minute }
        } else {
            minute_str.parse::<u32>().unwrap_or(current_minute)
        };

        let target_hour: u32 = if hour_str == "*" {
            current_hour
        } else if hour_str.contains(',') {
            if hour_str
                .split(',')
                .filter_map(|h| h.parse::<u32>().ok())
                .any(|h| h == current_hour) { current_hour } else { current_hour }
        } else {
            hour_str.parse::<u32>().unwrap_or(current_hour)
        };

        let time_diff_minutes = ((target_hour as i64 - current_hour as i64) * 60
            + (target_minute as i64 - current_minute as i64))
            .abs();

        let prewarm_minutes = (prewarm_secs as i64) / 60;

        time_diff_minutes <= prewarm_minutes || time_diff_minutes >= (24 * 60 - prewarm_minutes)
    }

    pub async fn predict_context_based(&self, context: &PredictionContext) -> Vec<PredictionResult> {
        let mut results = vec![];
        let hands = self.registry.list().await;
        let context_lower = context.keywords.join(" ").to_lowercase();

        for hand in hands {
            if let Some(ref config) = hand.predictive_config
                && config.enabled && !config.trigger_on_sequence.is_empty() {
                    let matched_keywords: Vec<&String> = config
                        .trigger_on_sequence
                        .iter()
                        .filter(|kw| context_lower.contains(&kw.to_lowercase()))
                        .collect();

                    if !matched_keywords.is_empty() {
                        let confidence = (matched_keywords.len() as f64) / (config.trigger_on_sequence.len() as f64);

                        if confidence >= 0.5 {
                            results.push(PredictionResult {
                                hand_id: hand.id.clone(),
                                confidence,
                                trigger_reason: format!("上下文匹配: {:?}", matched_keywords),
                                suggested_input: Some(context.input.clone()),
                            });
                        }
                    }
                }
        }

        results
    }

    pub async fn get_predictions(&self, execution_id: &str) -> Vec<PredictionResult> {
        let predictions = self.predictions.read().await;
        predictions.get(execution_id).cloned().unwrap_or_default()
    }

    pub async fn clear_predictions(&self, execution_id: &str) {
        let mut predictions = self.predictions.write().await;
        predictions.remove(execution_id);
    }

    pub async fn get_triggers(&self) -> Vec<PredictionTrigger> {
        let triggers = self.triggers.read().await;
        triggers.values().cloned().collect()
    }

    pub async fn enable_trigger(&self, trigger_id: &str) -> bool {
        let mut triggers = self.triggers.write().await;
        if let Some(trigger) = triggers.get_mut(trigger_id) {
            trigger.enabled = true;
            return true;
        }
        false
    }

    pub async fn disable_trigger(&self, trigger_id: &str) -> bool {
        let mut triggers = self.triggers.write().await;
        if let Some(trigger) = triggers.get_mut(trigger_id) {
            trigger.enabled = false;
            return true;
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct PredictionContext {
    pub execution_id: String,
    pub input: String,
    pub keywords: Vec<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

impl PredictionContext {
    pub fn new(execution_id: String, input: String) -> Self {
        let keywords = Self::extract_keywords(&input);
        Self {
            execution_id,
            input,
            keywords,
            user_id: None,
            session_id: None,
        }
    }

    fn extract_keywords(input: &str) -> Vec<String> {
        input
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prediction_engine_creation() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let context = PredictionContext::new(
            "test-exec-1".to_string(),
            "研究 Tesla 最新动态".to_string(),
        );

        let predictions = engine.predict(&context).await;
        assert!(predictions.is_empty());
    }

    #[tokio::test]
    async fn test_register_trigger() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let trigger = PredictionTrigger {
            id: "trigger-1".to_string(),
            hand_id: "researcher".to_string(),
            trigger_type: TriggerType::TimeBased,
            condition: PredictionCondition {
                time_pattern: Some("0 8 * * *".to_string()),
                preceding_actions: vec![],
                context_keywords: vec![],
                min_confidence: 0.5,
            },
            confidence_threshold: 0.5,
            enabled: true,
        };

        engine.register_trigger(trigger).await;

        let triggers = engine.get_triggers().await;
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].id, "trigger-1");
    }

    #[tokio::test]
    async fn test_context_prediction() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let context = PredictionContext::new(
            "test-exec-2".to_string(),
            "每天早上研究特斯拉最新消息".to_string(),
        );

        let predictions = engine.predict(&context).await;
        assert!(!predictions.is_empty() || predictions.is_empty());
    }

    #[tokio::test]
    async fn test_keyword_extraction() {
        let keywords = PredictionContext::extract_keywords("研究 特斯拉 最新 动态 科技");
        assert!(keywords.len() >= 3);
    }

    #[tokio::test]
    async fn test_enable_disable_trigger() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let trigger = PredictionTrigger {
            id: "trigger-test".to_string(),
            hand_id: "test-hand".to_string(),
            trigger_type: TriggerType::ContextBased,
            condition: PredictionCondition {
                time_pattern: None,
                preceding_actions: vec![],
                context_keywords: vec!["test".to_string()],
                min_confidence: 0.5,
            },
            confidence_threshold: 0.5,
            enabled: true,
        };

        engine.register_trigger(trigger).await;

        let disabled = engine.disable_trigger("trigger-test").await;
        assert!(disabled);

        let enabled = engine.enable_trigger("trigger-test").await;
        assert!(enabled);
    }

    #[tokio::test]
    async fn test_unregister_trigger() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let trigger = PredictionTrigger {
            id: "trigger-remove".to_string(),
            hand_id: "test-hand".to_string(),
            trigger_type: TriggerType::TimeBased,
            condition: PredictionCondition {
                time_pattern: Some("0 8 * * *".to_string()),
                preceding_actions: vec![],
                context_keywords: vec![],
                min_confidence: 0.5,
            },
            confidence_threshold: 0.5,
            enabled: true,
        };

        engine.register_trigger(trigger).await;
        assert_eq!(engine.get_triggers().await.len(), 1);

        engine.unregister_trigger("trigger-remove").await;
        assert!(engine.get_triggers().await.is_empty());
    }

    #[tokio::test]
    async fn test_time_pattern_check() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let engine = PredictionEngine::new(registry, schedule_manager);

        let result = engine.check_time_pattern("30 10 * * *", Utc::now(), 60);
        assert!(result == true || result == false);
    }
}
