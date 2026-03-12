use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationState {
    pub task_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub status: MetricDelegationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricDelegationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSnapshot {
    pub total_messages: u64,
    pub total_delegations: u64,
    pub active_count: usize,
    pub completed_count: u64,
    pub failed_count: u64,
    pub avg_response_time_ms: f64,
}

pub struct CollaborationMetrics {
    message_count: AtomicU64,
    delegation_count: AtomicU64,
    completed_count: AtomicU64,
    failed_count: AtomicU64,
    total_response_time_ms: AtomicU64,
    active_delegations: Arc<RwLock<HashMap<String, DelegationState>>>,
}

impl CollaborationMetrics {
    pub fn new() -> Self {
        Self {
            message_count: AtomicU64::new(0),
            delegation_count: AtomicU64::new(0),
            completed_count: AtomicU64::new(0),
            failed_count: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
            active_delegations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_message(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn record_delegation_start(&self, state: DelegationState) {
        self.delegation_count.fetch_add(1, Ordering::Relaxed);
        let mut active = self.active_delegations.write().await;
        active.insert(state.task_id.clone(), state);
    }

    pub async fn record_delegation_complete(&self, task_id: &str, response_time_ms: u64) {
        self.completed_count.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms.fetch_add(response_time_ms, Ordering::Relaxed);
        
        let mut active = self.active_delegations.write().await;
        if let Some(state) = active.get_mut(task_id) {
            state.status = MetricDelegationStatus::Completed;
            state.completed_at = Some(Utc::now());
        }
    }

    pub async fn record_delegation_failure(&self, task_id: &str) {
        self.failed_count.fetch_add(1, Ordering::Relaxed);
        
        let mut active = self.active_delegations.write().await;
        if let Some(state) = active.get_mut(task_id) {
            state.status = MetricDelegationStatus::Failed;
            state.completed_at = Some(Utc::now());
        }
    }

    pub async fn snapshot(&self) -> CollaborationSnapshot {
        let active_count = self.active_delegations.read().await.len();
        let completed = self.completed_count.load(Ordering::Relaxed);
        let total_response = self.total_response_time_ms.load(Ordering::Relaxed);
        
        let avg_response_time_ms = if completed > 0 {
            total_response as f64 / completed as f64
        } else {
            0.0
        };

        CollaborationSnapshot {
            total_messages: self.message_count.load(Ordering::Relaxed),
            total_delegations: self.delegation_count.load(Ordering::Relaxed),
            active_count,
            completed_count: completed,
            failed_count: self.failed_count.load(Ordering::Relaxed),
            avg_response_time_ms,
        }
    }

    pub async fn get_active_delegations(&self) -> Vec<DelegationState> {
        let active = self.active_delegations.read().await;
        active.values().cloned().collect()
    }

    pub async fn clear_completed(&self) {
        let mut active = self.active_delegations.write().await;
        active.retain(|_, state| {
            state.status == MetricDelegationStatus::Pending 
            || state.status == MetricDelegationStatus::InProgress
        });
    }
}

impl Default for CollaborationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = CollaborationMetrics::new();
        let snapshot = metrics.snapshot().await;
        
        assert_eq!(snapshot.total_messages, 0);
        assert_eq!(snapshot.total_delegations, 0);
    }

    #[tokio::test]
    async fn test_record_message() {
        let metrics = CollaborationMetrics::new();
        
        metrics.record_message().await;
        metrics.record_message().await;
        
        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.total_messages, 2);
    }

    #[tokio::test]
    async fn test_record_delegation() {
        let metrics = CollaborationMetrics::new();
        
        let state = DelegationState {
            task_id: "task_1".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            status: MetricDelegationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        metrics.record_delegation_start(state).await;
        
        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.total_delegations, 1);
        assert_eq!(snapshot.active_count, 1);
    }

    #[tokio::test]
    async fn test_record_delegation_complete() {
        let metrics = CollaborationMetrics::new();
        
        let state = DelegationState {
            task_id: "task_1".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            status: MetricDelegationStatus::InProgress,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        metrics.record_delegation_start(state).await;
        metrics.record_delegation_complete("task_1", 100).await;
        
        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.completed_count, 1);
        assert_eq!(snapshot.avg_response_time_ms, 100.0);
    }

    #[tokio::test]
    async fn test_record_delegation_failure() {
        let metrics = CollaborationMetrics::new();
        
        let state = DelegationState {
            task_id: "task_1".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            status: MetricDelegationStatus::InProgress,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        metrics.record_delegation_start(state).await;
        metrics.record_delegation_failure("task_1").await;
        
        let snapshot = metrics.snapshot().await;
        assert_eq!(snapshot.failed_count, 1);
    }

    #[tokio::test]
    async fn test_get_active_delegations() {
        let metrics = CollaborationMetrics::new();
        
        let state = DelegationState {
            task_id: "task_1".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            status: MetricDelegationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        metrics.record_delegation_start(state).await;
        
        let active = metrics.get_active_delegations().await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_clear_completed() {
        let metrics = CollaborationMetrics::new();
        
        let state1 = DelegationState {
            task_id: "task_1".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            status: MetricDelegationStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        
        let state2 = DelegationState {
            task_id: "task_2".to_string(),
            from_agent: "agent_1".to_string(),
            to_agent: "agent_3".to_string(),
            status: MetricDelegationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        metrics.record_delegation_start(state1).await;
        metrics.record_delegation_start(state2).await;
        
        metrics.clear_completed().await;
        
        let active = metrics.get_active_delegations().await;
        assert_eq!(active.len(), 1);
    }
}
