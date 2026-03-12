use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::task::{TaskInput, TaskRequest, TaskResult, TaskStatus, TaskType};
use crate::agent::Agent;
use crate::evo::registry::SharedSkillRegistry;
use super::hand::{Hand, HandRegistry, SkillCall};
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
    agent: Option<Arc<dyn Agent>>,
    skill_registry: Option<Arc<SharedSkillRegistry>>,
    optimizer: Option<Arc<super::optimizer::HandOptimizer>>,
    predictor: Option<Arc<super::predictor::PredictionEngine>>,
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
            agent: None,
            skill_registry: None,
            optimizer: None,
            predictor: None,
        }
    }

    pub fn with_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn has_agent(&self) -> bool {
        self.agent.is_some()
    }

    pub fn with_skill_registry(mut self, registry: Arc<SharedSkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    pub fn with_optimizer(mut self, optimizer: Arc<super::optimizer::HandOptimizer>) -> Self {
        self.optimizer = Some(optimizer);
        self
    }

    pub fn with_predictor(mut self, predictor: Arc<super::predictor::PredictionEngine>) -> Self {
        self.predictor = Some(predictor);
        self
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
                format!("Hand '{}' is disabled", hand_id),
                0,
            );
        }

        tracing::info!("Executing Hand: {} with task_id: {}", hand_id, ctx.task_id);
 
         let start_time = Instant::now();
 
         let execution_result = self.execute_hand_logic(&hand, &ctx).await;
         
         let execution_result = ExecutionResult {
             hand_id: hand_id.to_string(),
             task_id: ctx.task_id.clone(),
             success: execution_result.success,
             output: execution_result.data,
             error: execution_result.error,
             metrics: HashMap::new(),
             duration_ms: 0,
             timestamp: Utc::now(),
         };
         
         let _duration_ms = start_time.elapsed().as_millis() as u64;
 
         self.metrics_collector.record(&execution_result).await;
         
         self.record_execution(hand_id, execution_result.clone()).await;
 
         execution_result
     }
 
     pub async fn get_hand(&self, hand_id: &str) -> Option<Hand> {
         self.registry.get(hand_id).await
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
        let mut output = if let Some(agent) = &self.agent {
            let task = self.build_agent_task(hand, ctx).await;
            
            match agent.process(task).await {
                Ok(result) => {
                    self.convert_to_hand_output(result)
                }
                Err(e) => HandOutput {
                    success: false,
                    data: serde_json::json!({}),
                    error: Some(e.to_string()),
                }
            }
        } else {
            self.default_execution(hand, ctx).await
        };

        if !hand.skill_calls.is_empty() {
            let skill_results = self.execute_skill_calls(&hand.skill_calls, &output, ctx).await;
            output.data = serde_json::json!({
                "hand_result": output.data,
                "skill_results": skill_results,
            });
        }

        output
    }

    async fn execute_skill_calls(
        &self,
        skill_calls: &[SkillCall],
        hand_output: &HandOutput,
        _ctx: &ExecutionContext,
    ) -> Vec<serde_json::Value> {
        let mut results = Vec::new();

        if let Some(ref registry) = self.skill_registry {
            for call in skill_calls {
                let should_execute = self.evaluate_condition(&call.condition, hand_output).await;
                
                if should_execute
                    && let Some(skill) = registry.get_skill(&call.skill_id).await {
                        let input = self.render_template(&call.input_template, hand_output);
                        
                        let result = serde_json::json!({
                            "skill_id": call.skill_id,
                            "skill_name": skill.name,
                            "input": input,
                            "executed": true,
                        });
                        results.push(result);
                    }
            }
        }

        results
    }

    async fn evaluate_condition(&self, condition: &str, output: &HandOutput) -> bool {
        if condition.is_empty() {
            return true;
        }

        match condition {
            "always" => true,
            "on_success" => output.success,
            "on_failure" => !output.success,
            _ => {
                tracing::warn!("Unknown condition: {}", condition);
                false
            }
        }
    }

    fn render_template(&self, template: &str, output: &HandOutput) -> String {
        let json_str = serde_json::to_string(&output.data).unwrap_or_default();
        template.replace("{{output}}", &json_str)
    }

    async fn build_agent_task(&self, hand: &Hand, ctx: &ExecutionContext) -> TaskRequest {
        let input_content = ctx.input.as_str().unwrap_or("");
        
        let content = if hand.system_prompt.is_empty() {
            input_content.to_string()
        } else {
            format!("System: {}\n\nUser: {}", hand.system_prompt, input_content)
        };

        TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text { content },
        )
    }

    fn convert_to_hand_output(&self, result: TaskResult) -> HandOutput {
        use crate::task::TaskOutput;
        
        let (data, error) = match result.output {
            Some(TaskOutput::Text { content }) => {
                (serde_json::json!({ "text": content }), None)
            }
            Some(TaskOutput::ToolResult { result }) => {
                (result, None)
            }
            Some(TaskOutput::Code { language, code }) => {
                (serde_json::json!({ "language": language, "code": code }), None)
            }
            Some(TaskOutput::Data { data }) => {
                (data, None)
            }
            Some(TaskOutput::Message { message }) => {
                (serde_json::json!({ "message": message }), None)
            }
            Some(TaskOutput::Multiple { outputs }) => {
                (serde_json::json!({ "outputs": outputs }), None)
            }
            Some(TaskOutput::SearchResult { results }) => {
                (serde_json::json!({ "results": results }), None)
            }
            None => {
                (serde_json::json!({}), result.error.clone())
            }
        };

        HandOutput {
            success: result.status == TaskStatus::Completed,
            data,
            error,
        }
    }

    async fn default_execution(&self, hand: &Hand, ctx: &ExecutionContext) -> HandOutput {
        let output_data = serde_json::json!({
            "hand_id": hand.id,
            "name": hand.name,
            "description": hand.description,
            "input": ctx.input,
            "message": format!("Executed hand: {} (default mode)", hand.name)
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

    pub async fn update_hand_state(&self, hand_id: &str, success: bool, output: Option<String>) {
        if let Some(mut hand) = self.registry.get(hand_id).await {
            hand.state.execution_count += 1;
            hand.state.last_execution = Some(Utc::now());
            hand.state.last_output = output;
            
            if success {
                hand.state.consecutive_failures = 0;
                hand.state.status = super::hand::HandStatus::Active;
            } else {
                hand.state.consecutive_failures += 1;
                if hand.state.consecutive_failures >= hand.execution_config.max_retries {
                    hand.state.status = super::hand::HandStatus::Failed;
                }
            }
            
            let _ = self.registry.update(hand).await;
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
    use super::super::hand::HandCategory;

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
        assert!(result.error.unwrap().contains("disabled"));
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
