use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandLearningRecord {
    pub id: String,
    pub hand_id: String,
    pub execution_id: String,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
    pub skill_calls: Vec<SkillCallRecord>,
    pub timestamp: DateTime<Utc>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCallRecord {
    pub skill_id: String,
    pub input: String,
    pub output: serde_json::Value,
    pub success: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandExecutionAnalytics {
    pub hand_id: String,
    pub total_executions: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub avg_duration_ms: f64,
    pub failure_patterns: Vec<FailurePattern>,
    pub success_patterns: Vec<SuccessPattern>,
    pub skill_effectiveness: HashMap<String, SkillEffectiveness>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_type: String,
    pub frequency: u32,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub pattern_type: String,
    pub frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffectiveness {
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub call_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub hand_id: String,
    pub suggestion_type: OptimizationType,
    pub description: String,
    pub confidence: f64,
    pub auto_applicable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    RetryStrategy,
    TimeoutAdjustment,
    SkillCombination,
    ParameterTuning,
}

pub struct HandOptimizer {
    records: Arc<RwLock<HashMap<String, Vec<HandLearningRecord>>>>,
    execution_counts: Arc<RwLock<HashMap<String, u32>>>,
}

impl HandOptimizer {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            execution_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_execution(&self, record: HandLearningRecord) {
        let hand_id = record.hand_id.clone();
        
        let mut records = self.records.write().await;
        records
            .entry(hand_id.clone())
            .or_insert_with(Vec::new)
            .push(record);

        let mut counts = self.execution_counts.write().await;
        let count = counts.entry(hand_id).or_insert(0);
        *count += 1;
    }

    pub async fn get_records(&self, hand_id: &str) -> Vec<HandLearningRecord> {
        let records = self.records.read().await;
        records.get(hand_id).cloned().unwrap_or_default()
    }

    pub async fn should_optimize(&self, hand_id: &str, interval: u32) -> bool {
        let counts = self.execution_counts.read().await;
        counts.get(hand_id).map(|c| *c > 0 && *c % interval == 0).unwrap_or(false)
    }

    pub async fn generate_suggestions(&self, hand_id: &str) -> Vec<OptimizationSuggestion> {
        let records = self.get_records(hand_id).await;
        if records.is_empty() {
            return vec![];
        }

        let mut suggestions = vec![];

        let total = records.len() as u32;
        let successes = records.iter().filter(|r| r.success).count() as u32;
        let failures = total - successes;
        let failure_rate = failures as f64 / total as f64;

        if failure_rate > 0.5 {
            suggestions.push(OptimizationSuggestion {
                hand_id: hand_id.to_string(),
                suggestion_type: OptimizationType::RetryStrategy,
                description: format!("失败率 {:.1}% 超过50%, 建议增加重试次数", failure_rate * 100.0),
                confidence: 0.9,
                auto_applicable: true,
            });
        }

        let total_duration: u64 = records.iter().map(|r| r.duration_ms).sum();
        let avg_duration = total_duration as f64 / total as f64;

        if avg_duration > 60000.0 {
            suggestions.push(OptimizationSuggestion {
                hand_id: hand_id.to_string(),
                suggestion_type: OptimizationType::TimeoutAdjustment,
                description: format!("平均执行时间 {:.1}s, 建议增加超时时间", avg_duration / 1000.0),
                confidence: 0.8,
                auto_applicable: true,
            });
        }

        let mut skill_stats: HashMap<String, (u32, u32)> = HashMap::new();
        for record in &records {
            for skill in &record.skill_calls {
                let entry = skill_stats.entry(skill.skill_id.clone()).or_insert((0, 0));
                entry.0 += 1;
                if skill.success {
                    entry.1 += 1;
                }
            }
        }

        for (skill_id, (total_calls, success_calls)) in skill_stats {
            if total_calls > 0 {
                let success_rate = success_calls as f64 / total_calls as f64;
                if success_rate < 0.3 {
                    suggestions.push(OptimizationSuggestion {
                        hand_id: hand_id.to_string(),
                        suggestion_type: OptimizationType::SkillCombination,
                        description: format!(
                            "Skill {} 成功率仅 {:.1}%, 建议优化调用条件",
                            skill_id,
                            success_rate * 100.0
                        ),
                        confidence: 0.7,
                        auto_applicable: false,
                    });
                }
            }
        }

        suggestions
    }

    pub async fn generate_analytics(&self, hand_id: &str) -> Option<HandExecutionAnalytics> {
        let records = self.get_records(hand_id).await;
        if records.is_empty() {
            return None;
        }

        let total = records.len() as u32;
        let successes = records.iter().filter(|r| r.success).count() as u32;
        let failures = total - successes;

        let total_duration: u64 = records.iter().map(|r| r.duration_ms).sum();
        let avg_duration_ms = total_duration as f64 / total as f64;

        let mut failure_patterns = Vec::new();
        let mut success_patterns = Vec::new();

        let error_types: HashMap<String, u32> = records
            .iter()
            .filter(|r| !r.success)
            .filter_map(|r| r.error_message.clone())
            .fold(HashMap::new(), |mut acc, msg| {
                *acc.entry(msg).or_insert(0) += 1;
                acc
            });

        for (msg, count) in error_types {
            failure_patterns.push(FailurePattern {
                pattern_type: msg,
                frequency: count,
                suggested_fix: "请检查错误原因并调整执行策略".to_string(),
            });
        }

        if successes > 0 {
            success_patterns.push(SuccessPattern {
                pattern_type: "successful_execution".to_string(),
                frequency: successes,
            });
        }

        let mut skill_effectiveness = HashMap::new();
        let mut skill_durations: HashMap<String, (u64, u32)> = HashMap::new();
        let mut skill_successes: HashMap<String, u32> = HashMap::new();

        for record in &records {
            for skill in &record.skill_calls {
                let dur_entry = skill_durations.entry(skill.skill_id.clone()).or_insert((0, 0));
                dur_entry.0 += skill.duration_ms;
                dur_entry.1 += 1;

                if skill.success {
                    *skill_successes.entry(skill.skill_id.clone()).or_insert(0) += 1;
                }
            }
        }

        for (skill_id, (total_dur, call_count)) in skill_durations {
            let success_count = skill_successes.get(&skill_id).copied().unwrap_or(0);
            skill_effectiveness.insert(
                skill_id.clone(),
                SkillEffectiveness {
                    success_rate: if call_count > 0 {
                        success_count as f64 / call_count as f64
                    } else {
                        0.0
                    },
                    avg_duration_ms: if call_count > 0 {
                        total_dur as f64 / call_count as f64
                    } else {
                        0.0
                    },
                    call_count,
                },
            );
        }

        Some(HandExecutionAnalytics {
            hand_id: hand_id.to_string(),
            total_executions: total,
            success_count: successes,
            failure_count: failures,
            avg_duration_ms,
            failure_patterns,
            success_patterns,
            skill_effectiveness,
        })
    }
}

impl Default for HandOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_retrieve() {
        let optimizer = HandOptimizer::new();

        let record = HandLearningRecord {
            id: "test-1".to_string(),
            hand_id: "test-hand".to_string(),
            execution_id: "exec-1".to_string(),
            input: "test input".to_string(),
            output: "test output".to_string(),
            success: true,
            duration_ms: 1000,
            skill_calls: vec![],
            timestamp: Utc::now(),
            error_message: None,
        };

        optimizer.record_execution(record).await;

        let records = optimizer.get_records("test-hand").await;
        assert_eq!(records.len(), 1);
        assert!(records[0].success);
    }

    #[tokio::test]
    async fn test_failure_rate_suggestion() {
        let optimizer = HandOptimizer::new();

        for i in 0..10 {
            let record = HandLearningRecord {
                id: format!("test-{}", i),
                hand_id: "failing-hand".to_string(),
                execution_id: format!("exec-{}", i),
                input: "test".to_string(),
                output: "test".to_string(),
                success: i < 3,
                duration_ms: 100,
                skill_calls: vec![],
                timestamp: Utc::now(),
                error_message: if i >= 3 { Some("error".to_string()) } else { None },
            };
            optimizer.record_execution(record).await;
        }

        let suggestions = optimizer.generate_suggestions("failing-hand").await;
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| matches!(s.suggestion_type, OptimizationType::RetryStrategy)));
    }

    #[tokio::test]
    async fn test_timeout_suggestion() {
        let optimizer = HandOptimizer::new();

        for i in 0..5 {
            let record = HandLearningRecord {
                id: format!("test-{}", i),
                hand_id: "slow-hand".to_string(),
                execution_id: format!("exec-{}", i),
                input: "test".to_string(),
                output: "test".to_string(),
                success: true,
                duration_ms: 70000,
                skill_calls: vec![],
                timestamp: Utc::now(),
                error_message: None,
            };
            optimizer.record_execution(record).await;
        }

        let suggestions = optimizer.generate_suggestions("slow-hand").await;
        assert!(suggestions.iter().any(|s| matches!(s.suggestion_type, OptimizationType::TimeoutAdjustment)));
    }

    #[tokio::test]
    async fn test_skill_effectiveness() {
        let optimizer = HandOptimizer::new();

        let record = HandLearningRecord {
            id: "test-1".to_string(),
            hand_id: "skill-hand".to_string(),
            execution_id: "exec-1".to_string(),
            input: "test".to_string(),
            output: "test".to_string(),
            success: true,
            duration_ms: 1000,
            skill_calls: vec![
                SkillCallRecord {
                    skill_id: "skill-a".to_string(),
                    input: "input".to_string(),
                    output: serde_json::json!({"result": "ok"}),
                    success: true,
                    duration_ms: 100,
                },
                SkillCallRecord {
                    skill_id: "skill-b".to_string(),
                    input: "input".to_string(),
                    output: serde_json::json!({"error": "failed"}),
                    success: false,
                    duration_ms: 50,
                },
            ],
            timestamp: Utc::now(),
            error_message: None,
        };

        optimizer.record_execution(record).await;

        let analytics = optimizer.generate_analytics("skill-hand").await;
        assert!(analytics.is_some());

        let analytics = analytics.unwrap();
        assert!(analytics.skill_effectiveness.contains_key("skill-a"));
        assert!(analytics.skill_effectiveness.contains_key("skill-b"));
    }

    #[tokio::test]
    async fn test_should_optimize() {
        let optimizer = HandOptimizer::new();

        for i in 0..10 {
            let record = HandLearningRecord {
                id: format!("test-{}", i),
                hand_id: "opt-hand".to_string(),
                execution_id: format!("exec-{}", i),
                input: "test".to_string(),
                output: "test".to_string(),
                success: true,
                duration_ms: 100,
                skill_calls: vec![],
                timestamp: Utc::now(),
                error_message: None,
            };
            optimizer.record_execution(record).await;
        }

        let should_opt = optimizer.should_optimize("opt-hand", 10).await;
        assert!(should_opt);
    }
}
