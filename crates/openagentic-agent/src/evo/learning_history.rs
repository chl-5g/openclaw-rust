//! Learning History - 学习历史记录
//!
//! 跟踪任务学习历史，检测跨任务重复模式

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use tokio::sync::RwLock;

use super::pattern_analyzer::{TaskPattern, ToolCallPattern};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecord {
    pub id: String,
    pub task_id: String,
    pub pattern: TaskPattern,
    pub learning_type: LearningType,
    pub success: bool,
    pub created_at: DateTime<Utc>,
    pub task_input: String,
    pub task_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearningType {
    SuccessPattern,
    FailurePattern,
    Improvement,
    NewSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringPattern {
    pub id: String,
    pub category: String,
    pub tool_sequence: Vec<ToolCallPattern>,
    pub occurrence_count: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub avg_success_rate: f64,
    pub suggested_skill_name: Option<String>,
}

#[derive(Debug)]
pub struct LearningHistory {
    records: RwLock<HashMap<String, LearningRecord>>,
    task_patterns: RwLock<HashMap<String, Vec<String>>>,
    category_index: RwLock<HashMap<String, HashSet<String>>>,
    recurring_patterns: RwLock<Vec<RecurringPattern>>,
    config: HistoryConfig,
}

#[derive(Debug, Clone)]
pub struct HistoryConfig {
    pub max_records: usize,
    pub min_occurrences_for_recurring: u32,
    pub pattern_similarity_threshold: f64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_records: 10000,
            min_occurrences_for_recurring: 3,
            pattern_similarity_threshold: 0.8,
        }
    }
}

impl Default for LearningHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningHistory {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            task_patterns: RwLock::new(HashMap::new()),
            category_index: RwLock::new(HashMap::new()),
            recurring_patterns: RwLock::new(Vec::new()),
            config: HistoryConfig::default(),
        }
    }

    pub fn with_config(config: HistoryConfig) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            task_patterns: RwLock::new(HashMap::new()),
            category_index: RwLock::new(HashMap::new()),
            recurring_patterns: RwLock::new(Vec::new()),
            config,
        }
    }

    pub async fn add_record(&self, record: LearningRecord) {
        let record_id = record.id.clone();
        let task_id = record.task_id.clone();
        let category = record.pattern.task_category.clone();

        self.records.write().await.insert(record_id.clone(), record);

        {
            let mut patterns = self.task_patterns.write().await;
            patterns.entry(task_id).or_insert_with(Vec::new).push(record_id.clone());
        }

        {
            let mut index = self.category_index.write().await;
            index.entry(category).or_insert_with(HashSet::new).insert(record_id);
        }

        self.cleanup_old_records().await;
    }

    pub async fn record_success(&self, task_id: &str, pattern: TaskPattern, input: &str, output: Option<&str>) {
        let record = LearningRecord {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            pattern,
            learning_type: LearningType::SuccessPattern,
            success: true,
            created_at: Utc::now(),
            task_input: input.to_string(),
            task_output: output.map(String::from),
        };
        self.add_record(record).await;
    }

    pub async fn record_failure(&self, task_id: &str, pattern: TaskPattern, input: &str, error: &str) {
        let record = LearningRecord {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            pattern,
            learning_type: LearningType::FailurePattern,
            success: false,
            created_at: Utc::now(),
            task_input: input.to_string(),
            task_output: Some(error.to_string()),
        };
        self.add_record(record).await;
    }

    pub async fn get_records_by_task(&self, task_id: &str) -> Vec<LearningRecord> {
        let task_patterns = self.task_patterns.read().await;
        let records = self.records.read().await;
        
        task_patterns
            .get(task_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| records.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn get_records_by_category(&self, category: &str) -> Vec<LearningRecord> {
        let category_index = self.category_index.read().await;
        let records = self.records.read().await;
        
        category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| records.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn get_all_records(&self) -> Vec<LearningRecord> {
        let records = self.records.read().await;
        records.values().cloned().collect()
    }

    pub async fn get_records_count(&self) -> usize {
        self.records.read().await.len()
    }

    pub async fn detect_recurring(&self) -> Vec<RecurringPattern> {
        let mut recurring = Vec::new();
        
        let categories: Vec<String> = {
            let index = self.category_index.read().await;
            index.keys().cloned().collect()
        };
        
        for category_name in categories {
            let record_ids: Vec<String> = {
                let index = self.category_index.read().await;
                index.get(&category_name).cloned().map(|s| s.into_iter().collect()).unwrap_or_default()
            };
            
            if record_ids.len() < self.config.min_occurrences_for_recurring as usize {
                continue;
            }
            
            let mut tool_sequences: Vec<Vec<ToolCallPattern>> = Vec::new();
            let mut timestamps: Vec<DateTime<Utc>> = Vec::new();
            let mut success_count = 0usize;
            
            let records = self.records.read().await;
            for id in &record_ids {
                if let Some(record) = records.get(id) {
                    tool_sequences.push(record.pattern.tool_sequence.clone());
                    timestamps.push(record.created_at);
                    if record.success {
                        success_count += 1;
                    }
                }
            }
            drop(records);
            
            if tool_sequences.is_empty() {
                continue;
            }
            
            let is_similar = self.check_sequence_similarity(&tool_sequences);
            
            if is_similar {
                let avg_success_rate = success_count as f64 / tool_sequences.len() as f64;
                
                recurring.push(RecurringPattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    category: category_name,
                    tool_sequence: tool_sequences.first().cloned().unwrap_or_default(),
                    occurrence_count: tool_sequences.len() as u32,
                    first_seen: *timestamps.first().unwrap_or(&Utc::now()),
                    last_seen: *timestamps.last().unwrap_or(&Utc::now()),
                    avg_success_rate,
                    suggested_skill_name: None,
                });
            }
        }
        
        let mut write = self.recurring_patterns.write().await;
        *write = recurring.clone();
        
        recurring
    }

    fn check_sequence_similarity(&self, sequences: &[Vec<ToolCallPattern>]) -> bool {
        if sequences.len() < 2 {
            return false;
        }
        
        let reference = &sequences[0];
        
        let mut similar_count = 0usize;
        
        for sequence in sequences.iter().skip(1) {
            if self.sequence_similarity(reference, sequence) >= self.config.pattern_similarity_threshold {
                similar_count += 1;
            }
        }
        
        let required_similar = (sequences.len() - 1) / 2;
        similar_count >= required_similar
    }

    fn sequence_similarity(&self, seq1: &[ToolCallPattern], seq2: &[ToolCallPattern]) -> f64 {
        if seq1.is_empty() && seq2.is_empty() {
            return 1.0;
        }
        
        if seq1.is_empty() || seq2.is_empty() {
            return 0.0;
        }
        
        let max_len = seq1.len().max(seq2.len());
        let min_len = seq1.len().min(seq2.len());
        
        let mut matches = 0usize;
        
        for i in 0..min_len {
            if seq1[i].tool_name == seq2[i].tool_name {
                matches += 1;
            }
        }
        
        matches as f64 / max_len as f64
    }

    pub async fn get_recurring_patterns(&self) -> Vec<RecurringPattern> {
        self.recurring_patterns.read().await.clone()
    }

    pub async fn update_pattern_suggestion(&self, pattern_id: &str, skill_name: String) {
        let mut write = self.recurring_patterns.write().await;
        if let Some(pattern) = write.iter_mut().find(|p| p.id == pattern_id) {
            pattern.suggested_skill_name = Some(skill_name);
        }
    }

    async fn cleanup_old_records(&self) {
        let count = {
            let records = self.records.read().await;
            records.len()
        };
        
        if count > self.config.max_records {
            let to_remove = count - self.config.max_records;
            
            let mut sorted: Vec<(String, DateTime<Utc>)> = {
                let records = self.records.read().await;
                records.iter().map(|(k, v)| (k.clone(), v.created_at)).collect()
            };
            sorted.sort_by(|a, b| a.1.cmp(&b.1));
            
            for (id, _) in sorted.iter().take(to_remove) {
                let task_id: Option<String>;
                let category: Option<String>;
                
                {
                    let records = self.records.read().await;
                    if let Some(record) = records.get(id) {
                        task_id = Some(record.task_id.clone());
                        category = Some(record.pattern.task_category.clone());
                    } else {
                        continue;
                    }
                }
                
                self.records.write().await.remove(id);
                
                if let Some(tid) = task_id {
                    let mut patterns = self.task_patterns.write().await;
                    if let Some(ids) = patterns.get_mut(&tid) {
                        ids.retain(|p| p != id);
                    }
                }
                
                if let Some(cat) = category {
                    let mut index = self.category_index.write().await;
                    if let Some(set) = index.get_mut(&cat) {
                        set.remove(id);
                    }
                }
            }
        }
    }

    pub async fn clear(&self) {
        self.records.write().await.clear();
        self.task_patterns.write().await.clear();
        self.category_index.write().await.clear();
        self.recurring_patterns.write().await.clear();
    }

    pub async fn get_statistics(&self) -> HistoryStatistics {
        let records = self.records.read().await;
        let total = records.len();
        let success = records.values().filter(|r| r.success).count();
        let failure = total - success;
        
        let index = self.category_index.read().await;
        let mut category_counts: HashMap<String, usize> = HashMap::new();
        for (cat, set) in index.iter() {
            category_counts.insert(cat.clone(), set.len());
        }
        
        HistoryStatistics {
            total_records: total,
            success_count: success,
            failure_count: failure,
            category_counts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStatistics {
    pub total_records: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub category_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pattern(task_id: &str, category: &str) -> TaskPattern {
        TaskPattern {
            id: uuid::Uuid::new_v4().to_string(),
            task_category: category.to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "search".to_string(),
                    param_schema: std::collections::HashMap::new(),
                    result_schema: std::collections::HashMap::new(),
                },
            ],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.7,
            source_task_id: task_id.to_string(),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_record_success() {
        let history = LearningHistory::new();
        
        let pattern = create_test_pattern("task-1", "search");
        history.record_success("task-1", pattern, "search for rust", Some("found results")).await;
        
        assert_eq!(history.get_records_count().await, 1);
        
        let records = history.get_records_by_task("task-1").await;
        assert_eq!(records.len(), 1);
        assert!(records[0].success);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let history = LearningHistory::new();
        
        let pattern = create_test_pattern("task-1", "api_call");
        history.record_failure("task-1", pattern, "call api", "connection timeout").await;
        
        assert_eq!(history.get_records_count().await, 1);
        
        let records = history.get_records_by_task("task-1").await;
        assert_eq!(records.len(), 1);
        assert!(!records[0].success);
    }

    #[tokio::test]
    async fn test_get_records_by_category() {
        let history = LearningHistory::new();
        
        let pattern1 = create_test_pattern("task-1", "search");
        history.record_success("task-1", pattern1, "search1", None).await;
        
        let pattern2 = create_test_pattern("task-2", "search");
        history.record_success("task-2", pattern2, "search2", None).await;
        
        let pattern3 = create_test_pattern("task-3", "api_call");
        history.record_success("task-3", pattern3, "api call", None).await;
        
        let search_records = history.get_records_by_category("search").await;
        assert_eq!(search_records.len(), 2);
        
        let api_records = history.get_records_by_category("api_call").await;
        assert_eq!(api_records.len(), 1);
    }

    #[tokio::test]
    async fn test_detect_recurring() {
        let history = LearningHistory::new();
        
        for i in 0..5 {
            let pattern = create_test_pattern(&format!("task-{}", i), "search");
            history.record_success(&format!("task-{}", i), pattern, "search query", None).await;
        }
        
        let recurring = history.detect_recurring().await;
        
        assert!(!recurring.is_empty());
        
        let search_pattern = recurring.iter().find(|p| p.category == "search");
        assert!(search_pattern.is_some());
        assert_eq!(search_pattern.unwrap().occurrence_count, 5);
    }

    #[test]
    fn test_sequence_similarity() {
        let history = LearningHistory::new();
        
        let seq1 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
            ToolCallPattern {
                tool_name: "fetch".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let seq2 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
            ToolCallPattern {
                tool_name: "fetch".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let seq3 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
            ToolCallPattern {
                tool_name: "different".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let similarity_12 = history.sequence_similarity(&seq1, &seq2);
        assert!(similarity_12 > 0.9);
        
        let similarity_13 = history.sequence_similarity(&seq1, &seq3);
        assert!(similarity_13 < 0.9);
    }

    #[test]
    fn test_check_sequence_similarity() {
        let history = LearningHistory::new();
        
        let seq1 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let seq2 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let seq3 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];
        
        let sequences = vec![seq1, seq2, seq3];
        let is_similar = history.check_sequence_similarity(&sequences);
        assert!(is_similar);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let history = LearningHistory::new();
        
        let pattern1 = create_test_pattern("task-1", "search");
        history.record_success("task-1", pattern1, "query1", None).await;
        
        let pattern2 = create_test_pattern("task-2", "search");
        history.record_success("task-2", pattern2, "query2", None).await;
        
        let pattern3 = create_test_pattern("task-3", "api_call");
        history.record_failure("task-3", pattern3, "query3", "error").await;
        
        let stats = history.get_statistics().await;
        
        assert_eq!(stats.total_records, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.category_counts.get("search"), Some(&2));
        assert_eq!(stats.category_counts.get("api_call"), Some(&1));
    }

    #[tokio::test]
    async fn test_clear() {
        let history = LearningHistory::new();
        
        let pattern = create_test_pattern("task-1", "search");
        history.record_success("task-1", pattern, "query", None).await;
        
        assert_eq!(history.get_records_count().await, 1);
        
        history.clear().await;
        
        assert_eq!(history.get_records_count().await, 0);
    }

    #[tokio::test]
    async fn test_update_pattern_suggestion() {
        let history = LearningHistory::new();
        
        for i in 0..3 {
            let pattern = create_test_pattern(&format!("task-{}", i), "search");
            history.record_success(&format!("task-{}", i), pattern, "query", None).await;
        }
        
        history.detect_recurring().await;
        
        let patterns = history.get_recurring_patterns().await;
        if let Some(pattern) = patterns.first() {
            let pattern_id = pattern.id.clone();
            history.update_pattern_suggestion(&pattern_id, "search_skill".to_string()).await;
            
            let updated = history.get_recurring_patterns().await;
            let updated_pattern = updated.iter().find(|p| p.id == pattern_id);
            assert!(updated_pattern.is_some());
            assert_eq!(updated_pattern.unwrap().suggested_skill_name, Some("search_skill".to_string()));
        }
    }

    #[tokio::test]
    async fn test_get_all_records() {
        let history = LearningHistory::new();
        
        let pattern1 = create_test_pattern("task-1", "search");
        history.record_success("task-1", pattern1, "query1", None).await;
        
        let pattern2 = create_test_pattern("task-2", "api_call");
        history.record_success("task-2", pattern2, "query2", None).await;
        
        let all = history.get_all_records().await;
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_with_config() {
        let config = HistoryConfig {
            max_records: 100,
            min_occurrences_for_recurring: 2,
            pattern_similarity_threshold: 0.5,
        };
        
        let history = LearningHistory::with_config(config);
        
        assert_eq!(history.config.max_records, 100);
        assert_eq!(history.config.min_occurrences_for_recurring, 2);
        assert_eq!(history.config.pattern_similarity_threshold, 0.5);
    }

    #[tokio::test]
    async fn test_multiple_tasks_same_category() {
        let history = LearningHistory::new();
        
        for i in 0..10 {
            let pattern = create_test_pattern(&format!("task-{}", i), "file_operation");
            history.record_success(&format!("task-{}", i), pattern, &format!("file operation {}", i), None).await;
        }
        
        let records = history.get_records_by_category("file_operation").await;
        assert_eq!(records.len(), 10);
    }
}
