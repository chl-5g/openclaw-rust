use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::executor::ExecutionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandMetrics {
    pub hand_id: String,
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub min_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
    pub custom_metrics: HashMap<String, f64>,
}

impl HandMetrics {
    pub fn new(hand_id: String) -> Self {
        Self {
            hand_id,
            total_runs: 0,
            successful_runs: 0,
            failed_runs: 0,
            success_rate: 0.0,
            avg_duration_ms: 0.0,
            min_duration_ms: None,
            max_duration_ms: None,
            last_run: None,
            last_success: None,
            last_failure: None,
            custom_metrics: HashMap::new(),
        }
    }

    fn update(&mut self, result: &ExecutionResult) {
        self.total_runs += 1;
        self.last_run = Some(result.timestamp);

        if result.success {
            self.successful_runs += 1;
            self.last_success = Some(result.timestamp);
        } else {
            self.failed_runs += 1;
            self.last_failure = Some(result.timestamp);
        }

        self.success_rate = if self.total_runs > 0 {
            self.successful_runs as f64 / self.total_runs as f64
        } else {
            0.0
        };

        let total_duration = self.avg_duration_ms * (self.total_runs as f64 - 1.0);
        self.avg_duration_ms = (total_duration + result.duration_ms as f64) / self.total_runs as f64;

        if let Some(min) = self.min_duration_ms {
            self.min_duration_ms = Some(min.min(result.duration_ms));
        } else {
            self.min_duration_ms = Some(result.duration_ms);
        }

        if let Some(max) = self.max_duration_ms {
            self.max_duration_ms = Some(max.max(result.duration_ms));
        } else {
            self.max_duration_ms = Some(result.duration_ms);
        }

        for (key, value) in &result.metrics {
            *self.custom_metrics.entry(key.clone()).or_insert(0.0) += value;
        }
    }
}

pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, HandMetrics>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record(&self, result: &ExecutionResult) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(result.hand_id.clone()).or_insert_with(|| {
            HandMetrics::new(result.hand_id.clone())
        });
        entry.update(result);
    }

    pub async fn get(&self, hand_id: &str) -> Option<HandMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(hand_id).cloned()
    }

    pub async fn get_all(&self) -> Vec<HandMetrics> {
        let metrics = self.metrics.read().await;
        metrics.values().cloned().collect()
    }

    pub async fn reset(&self, hand_id: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.remove(hand_id);
    }

    pub async fn reset_all(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();
    }

    pub async fn calculate_brier_score(&self, predictions: &[(f64, bool)]) -> f64 {
        if predictions.is_empty() {
            return 0.0;
        }

        let sum: f64 = predictions
            .iter()
            .map(|(prediction, outcome)| {
                let outcome_value = if *outcome { 1.0 } else { 0.0 };
                (prediction - outcome_value).powi(2)
            })
            .sum();

        sum / predictions.len() as f64
    }

    pub async fn record_prediction(&self, hand_id: &str, prediction: f64, outcome: bool) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(hand_id.to_string()).or_insert_with(|| {
            HandMetrics::new(hand_id.to_string())
        });

        let brier_score = (prediction - if outcome { 1.0 } else { 0.0 }).powi(2);
        *entry.custom_metrics.entry("brier_score".to_string()).or_insert(0.0) = brier_score;
        *entry.custom_metrics.entry("predictions_count".to_string()).or_insert(0.0) += 1.0;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_success() {
        let collector = MetricsCollector::new();
        
        let result = ExecutionResult::success(
            "test".to_string(),
            "task_1".to_string(),
            serde_json::json!({"result": "ok"}),
            100,
        );
        
        collector.record(&result).await;
        let metrics = collector.get("test").await;
        
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.total_runs, 1);
        assert_eq!(m.successful_runs, 1);
        assert_eq!(m.success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let collector = MetricsCollector::new();
        
        let result = ExecutionResult::failure(
            "test".to_string(),
            "task_1".to_string(),
            "error".to_string(),
            50,
        );
        
        collector.record(&result).await;
        let metrics = collector.get("test").await;
        
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.total_runs, 1);
        assert_eq!(m.failed_runs, 1);
        assert_eq!(m.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_multiple_runs() {
        let collector = MetricsCollector::new();
        
        for i in 0..5 {
            let result = ExecutionResult::success(
                "test".to_string(),
                format!("task_{}", i),
                serde_json::json!({}),
                100 + i as u64,
            );
            collector.record(&result).await;
        }
        
        let metrics = collector.get("test").await.unwrap();
        assert_eq!(metrics.total_runs, 5);
        assert_eq!(metrics.success_rate, 1.0);
        assert_eq!(metrics.avg_duration_ms, 102.0);
    }

    #[tokio::test]
    async fn test_brier_score() {
        let collector = MetricsCollector::new();
        
        let predictions = vec![
            (0.8, true),
            (0.3, false),
            (0.6, true),
            (0.4, false),
        ];
        
        let score = collector.calculate_brier_score(&predictions).await;
        
        let expected: f64 = ((0.8f64 - 1.0) * (0.8f64 - 1.0) + (0.3f64 - 0.0) * (0.3f64 - 0.0) 
            + (0.6f64 - 1.0) * (0.6f64 - 1.0) + (0.4f64 - 0.0) * (0.4f64 - 0.0)) / 4.0;
        assert!((score - expected).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_reset() {
        let collector = MetricsCollector::new();
        
        let result = ExecutionResult::success("test".to_string(), "task_1".to_string(), serde_json::json!({}), 100);
        collector.record(&result).await;
        
        collector.reset("test").await;
        let metrics = collector.get("test").await;
        
        assert!(metrics.is_none());
    }

    #[tokio::test]
    async fn test_get_all() {
        let collector = MetricsCollector::new();
        
        collector.record(&ExecutionResult::success("h1".to_string(), "t1".to_string(), serde_json::json!({}), 100)).await;
        collector.record(&ExecutionResult::success("h2".to_string(), "t2".to_string(), serde_json::json!({}), 100)).await;
        
        let all = collector.get_all().await;
        assert_eq!(all.len(), 2);
    }
}
