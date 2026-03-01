use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::hand::{Hand, HandCategory, HandRegistry};
use super::schedule::ScheduleManager;
use super::metrics::MetricsCollector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub hand_id: String,
    pub task_id: String,
    pub input: serde_json::Value,
    pub scheduled_time: Option<chrono::DateTime<chrono::Utc>>,
    pub previous_results: Option<Vec<ExecutionResult>>,
    pub metadata: HashMap<String, String>,
}

impl ExecutionContext {
    pub fn new(hand_id: String) -> Self {
        Self {
            hand_id,
            task_id: uuid::Uuid::new_v4().to_string(),
            input: serde_json::json!({}),
            scheduled_time: None,
            previous_results: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    pub fn with_scheduled_time(mut self, time: chrono::DateTime<chrono::Utc>) -> Self {
        self.scheduled_time = Some(time);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub hand_id: String,
    pub task_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub metrics: HashMap<String, f64>,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ExecutionResult {
    pub fn success(hand_id: String, task_id: String, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            hand_id,
            task_id,
            success: true,
            output,
            error: None,
            metrics: HashMap::new(),
            duration_ms,
            timestamp: Utc::now(),
        }
    }

    pub fn failure(hand_id: String, task_id: String, error: String, duration_ms: u64) -> Self {
        Self {
            hand_id,
            task_id,
            success: false,
            output: serde_json::json!({}),
            error: Some(error),
            metrics: HashMap::new(),
            duration_ms,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub hand_id: String,
    pub action: String,
    pub description: String,
    pub prompt: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub status: ApprovalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

pub struct HandExecutor {
    registry: Arc<HandRegistry>,
    schedule_manager: Arc<ScheduleManager>,
    metrics_collector: Arc<MetricsCollector>,
    approval_requests: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    execution_history: Arc<RwLock<HashMap<String, Vec<ExecutionResult>>>>,
}

impl HandExecutor {
    pub fn new(
        registry: Arc<HandRegistry>,
        schedule_manager: Arc<ScheduleManager>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            registry,
            schedule_manager,
            metrics_collector,
            approval_requests: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn execute(&self, hand_id: &str, ctx: ExecutionContext) -> ExecutionResult {
        let hand = match self.registry.get(hand_id).await {
            Some(h) => h,
            None => {
                return ExecutionResult::failure(
                    hand_id.to_string(),
                    ctx.task_id,
                    format!("Hand '{}' not found", hand_id),
                    0,
                );
            }
        };

        if !hand.enabled {
            return ExecutionResult::failure(
                hand_id.to_string(),
                ctx.task_id,
                format!("Hand '{}' is not enabled", hand_id),
                0,
            );
        }

        let start = Instant::now();

        if let Some(approval_result) = self.check_guardrails(&hand, &ctx).await {
            return approval_result;
        }

        let output = self.execute_hand_logic(&hand, &ctx).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let result = if output.success {
            ExecutionResult::success(
                hand_id.to_string(),
                ctx.task_id,
                output.data,
                duration_ms,
            )
        } else {
            ExecutionResult::failure(
                hand_id.to_string(),
                ctx.task_id,
                output.error.unwrap_or_else(|| "Unknown error".to_string()),
                duration_ms,
            )
        };

        self.record_execution(hand_id, result.clone()).await;
        self.metrics_collector.record(&result).await;

        result
    }

    async fn check_guardrails(&self, hand: &Hand, ctx: &ExecutionContext) -> Option<ExecutionResult> {
        if hand.guardrails.is_empty() {
            return None;
        }

        for guardrail in &hand.guardrails {
            match &guardrail.action {
                super::hand::GuardrailAction::RequireApproval { prompt } => {
                    let request = ApprovalRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        hand_id: hand.id.clone(),
                        action: "sensitive_operation".to_string(),
                        description: guardrail.description.clone(),
                        prompt: prompt.clone(),
                        requested_at: Utc::now(),
                        status: ApprovalStatus::Pending,
                    };

                    let request_id = request.id.clone();
                    {
                        let mut approvals = self.approval_requests.write().await;
                        approvals.insert(request_id.clone(), request);
                    }

                    return Some(ExecutionResult::failure(
                        hand.id.clone(),
                        ctx.task_id.clone(),
                        format!("Approval required: {}", prompt),
                        0,
                    ));
                }
                super::hand::GuardrailAction::Block => {
                    return Some(ExecutionResult::failure(
                        hand.id.clone(),
                        ctx.task_id.clone(),
                        format!("Blocked: {}", guardrail.description),
                        0,
                    ));
                }
                super::hand::GuardrailAction::Log => {
                    tracing::info!("Guardrail log: {}", guardrail.description);
                }
            }
        }

        None
    }

    async fn execute_hand_logic(&self, hand: &Hand, ctx: &ExecutionContext) -> HandOutput {
        let output_data = serde_json::json!({
            "hand_id": hand.id,
            "name": hand.name,
            "description": hand.description,
            "input": ctx.input,
            "message": format!("Executed hand: {} (stub - integrate with Agent)", hand.name)
        });

        HandOutput {
            success: true,
            data: output_data,
            error: None,
        }
    }

    async fn record_execution(&self, hand_id: &str, result: ExecutionResult) {
        let mut history = self.execution_history.write().await;
        let entries = history.entry(hand_id.to_string()).or_insert_with(Vec::new);
        entries.push(result);

        if entries.len() > 100 {
            entries.remove(0);
        }
    }

    pub async fn get_execution_history(&self, hand_id: &str) -> Vec<ExecutionResult> {
        let history = self.execution_history.read().await;
        history.get(hand_id).cloned().unwrap_or_default()
    }

    pub async fn get_pending_approvals(&self) -> Vec<ApprovalRequest> {
        let approvals = self.approval_requests.read().await;
        approvals
            .values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    pub async fn approve(&self, request_id: &str) -> bool {
        let mut approvals = self.approval_requests.write().await;
        if let Some(request) = approvals.get_mut(request_id) {
            request.status = ApprovalStatus::Approved;
            true
        } else {
            false
        }
    }

    pub async fn reject(&self, request_id: &str) -> bool {
        let mut approvals = self.approval_requests.write().await;
        if let Some(request) = approvals.get_mut(request_id) {
            request.status = ApprovalStatus::Rejected;
            true
        } else {
            false
        }
    }
}

struct HandOutput {
    success: bool,
    data: serde_json::Value,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_not_found() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());

        let executor = HandExecutor::new(registry, schedule_manager, metrics);
        let ctx = ExecutionContext::new("nonexistent".to_string());

        let result = executor.execute("nonexistent", ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_disabled() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());

        let hand = Hand::new("test".to_string(), "Test".to_string(), "Test".to_string(), HandCategory::Custom);
        registry.register(hand).await;

        let executor = HandExecutor::new(registry.clone(), schedule_manager, metrics);
        let ctx = ExecutionContext::new("test".to_string());

        let result = executor.execute("test", ctx).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not enabled"));
    }

    #[tokio::test]
    async fn test_execute_success() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());

        let mut hand = Hand::new("test".to_string(), "Test".to_string(), "Test".to_string(), HandCategory::Custom);
        hand.enable();
        registry.register(hand).await;

        let executor = HandExecutor::new(registry.clone(), schedule_manager, metrics);
        let ctx = ExecutionContext::new("test".to_string());

        let result = executor.execute("test", ctx).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_approval() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());

        let executor = HandExecutor::new(registry, schedule_manager, metrics);

        let approvals = executor.get_pending_approvals().await;
        assert!(approvals.is_empty());
    }

    #[tokio::test]
    async fn test_execution_history() {
        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());

        let mut hand = Hand::new("test".to_string(), "Test".to_string(), "Test".to_string(), HandCategory::Custom);
        hand.enable();
        registry.register(hand).await;

        let executor = HandExecutor::new(registry.clone(), schedule_manager, metrics);
        
        let ctx = ExecutionContext::new("test".to_string());
        let _ = executor.execute("test", ctx).await;

        let history = executor.get_execution_history("test").await;
        assert_eq!(history.len(), 1);
    }
}
