//! Workflow Registry
//!
//! 用于管理和执行 Workflow Skill 定义的工作流

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::registry::DynamicSkill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub agent_id: String,
    pub action: String,
    #[serde(default)]
    pub input: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub on_success: Option<String>,
    #[serde(default)]
    pub on_failure: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub current_step: usize,
    pub status: WorkflowStatus,
    pub results: HashMap<String, serde_json::Value>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub type WorkflowEventCallback = Arc<dyn Fn(WorkflowEvent) + Send + Sync>;

#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    Started(String),
    StepStarted(String, usize),
    StepCompleted(String, usize, serde_json::Value),
    StepFailed(String, usize, String),
    Completed(String),
    Failed(String, String),
}

pub struct WorkflowRegistry {
    workflows: Arc<RwLock<HashMap<String, WorkflowDefinition>>>,
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    callback: Option<WorkflowEventCallback>,
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            callback: None,
        }
    }

    pub fn with_callback(mut self, callback: WorkflowEventCallback) -> Self {
        self.callback = Some(callback);
        self
    }

    pub async fn register(&self, workflow: WorkflowDefinition) -> Result<(), String> {
        let id = workflow.id.clone();
        
        if self.workflows.read().await.contains_key(&id) {
            return Err(format!("Workflow '{}' already registered", id));
        }

        self.workflows.write().await.insert(id.clone(), workflow);
        tracing::info!("Registered workflow: {}", id);
        Ok(())
    }

    pub async fn unregister(&self, id: &str) -> Result<(), String> {
        if self.workflows.write().await.remove(id).is_none() {
            return Err(format!("Workflow '{}' not found", id));
        }
        tracing::info!("Unregistered workflow: {}", id);
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Option<WorkflowDefinition> {
        self.workflows.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<WorkflowDefinition> {
        self.workflows.read().await.values().cloned().collect()
    }

    pub async fn apply_from_skill(&self, skill: &DynamicSkill) -> Result<(), String> {
        if !skill.is_workflow() {
            return Err("Skill is not a workflow type".to_string());
        }

        let workflow = self.parse_workflow_from_skill(skill)?;
        self.register(workflow).await?;

        tracing::info!("Applied workflow skill: {}", skill.name);
        Ok(())
    }

    fn parse_workflow_from_skill(&self, skill: &DynamicSkill) -> Result<WorkflowDefinition, String> {
        if let Some(instructions) = &skill.instructions {
            if let Some(workflow_yaml) = Self::extract_yaml_block(instructions) {
                return serde_yaml::from_str(&workflow_yaml)
                    .map_err(|e| format!("Failed to parse workflow: {}", e));
            }
        }

        Err("No workflow definition found in skill instructions".to_string())
    }

    fn extract_yaml_block(content: &str) -> Option<String> {
        let mut in_yaml_block = false;
        let mut yaml_lines = Vec::new();

        for line in content.lines() {
            if line.trim().starts_with("```yaml") {
                in_yaml_block = true;
                continue;
            }
            if line.trim() == "```" && in_yaml_block {
                break;
            }
            if in_yaml_block {
                yaml_lines.push(line);
            }
        }

        if yaml_lines.is_empty() {
            None
        } else {
            Some(yaml_lines.join("\n"))
        }
    }

    pub async fn start_execution(&self, workflow_id: &str) -> Result<String, String> {
        let workflow = self.get(workflow_id).await
            .ok_or_else(|| format!("Workflow '{}' not found", workflow_id))?;

        let execution_id = uuid::Uuid::new_v4().to_string();
        let execution = WorkflowExecution {
            workflow_id: workflow_id.to_string(),
            current_step: 0,
            status: WorkflowStatus::Running,
            results: HashMap::new(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        self.executions.write().await.insert(execution_id.clone(), execution);

        if let Some(callback) = &self.callback {
            callback(WorkflowEvent::Started(execution_id.clone()));
        }

        tracing::info!("Started workflow execution: {} for workflow: {}", execution_id, workflow_id);
        Ok(execution_id)
    }

    pub async fn execute_next_step(&self, execution_id: &str) -> Result<serde_json::Value, String> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;

        let workflow = self.workflows.read().await
            .get(&execution.workflow_id)
            .ok_or_else(|| format!("Workflow '{}' not found", execution.workflow_id))?
            .clone();
        
        drop(self.workflows.read().await);

        if execution.current_step >= workflow.steps.len() {
            execution.status = WorkflowStatus::Completed;
            execution.completed_at = Some(chrono::Utc::now());

            if let Some(callback) = &self.callback {
                callback(WorkflowEvent::Completed(execution_id.to_string()));
            }

            return Ok(serde_json::json!({ "status": "completed" }));
        }

        let step = &workflow.steps[execution.current_step];
        
        if let Some(callback) = &self.callback {
            callback(WorkflowEvent::StepStarted(execution_id.to_string(), execution.current_step));
        }

        let result = serde_json::json!({
            "agent_id": step.agent_id,
            "action": step.action,
            "input": step.input,
        });

        execution.results.insert(format!("step_{}", execution.current_step), result.clone());
        execution.current_step += 1;

        let callback_result = result.clone();
        if let Some(callback) = &self.callback {
            callback(WorkflowEvent::StepCompleted(execution_id.to_string(), execution.current_step - 1, callback_result));
        }

        Ok(result)
    }

    pub async fn get_execution(&self, execution_id: &str) -> Option<WorkflowExecution> {
        self.executions.read().await.get(execution_id).cloned()
    }

    pub async fn list_executions(&self) -> Vec<WorkflowExecution> {
        self.executions.read().await.values().cloned().collect()
    }

    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), String> {
        let mut executions = self.executions.write().await;
        let execution = executions.get_mut(execution_id)
            .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;

        execution.status = WorkflowStatus::Cancelled;
        
        if let Some(callback) = &self.callback {
            callback(WorkflowEvent::Failed(execution_id.to_string(), "Cancelled by user".to_string()));
        }

        tracing::info!("Cancelled workflow execution: {}", execution_id);
        Ok(())
    }

    pub fn get_skill_dir(skill: &DynamicSkill) -> Option<PathBuf> {
        skill.metadata.get("skill_dir")
            .map(PathBuf::from)
    }

    pub async fn reload_from_skills(&self, skills: Vec<DynamicSkill>) -> Result<(), String> {
        let mut workflows = self.workflows.write().await;
        workflows.clear();

        for skill in skills {
            if skill.is_workflow() {
                if let Ok(workflow) = self.parse_workflow_from_skill(&skill) {
                    workflows.insert(workflow.id.clone(), workflow);
                }
            }
        }

        tracing::info!("Reloaded {} workflows from skills", workflows.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_workflow(id: &str) -> WorkflowDefinition {
        WorkflowDefinition {
            id: id.to_string(),
            name: format!("Test Workflow {}", id),
            description: "Test workflow description".to_string(),
            steps: vec![
                WorkflowStep {
                    agent_id: "ceo".to_string(),
                    action: "plan".to_string(),
                    input: HashMap::new(),
                    output: None,
                    conditions: vec![],
                    on_success: Some("pm".to_string()),
                    on_failure: None,
                },
                WorkflowStep {
                    agent_id: "pm".to_string(),
                    action: "execute".to_string(),
                    input: HashMap::new(),
                    output: None,
                    conditions: vec![],
                    on_success: None,
                    on_failure: None,
                },
            ],
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_register_workflow() {
        let registry = WorkflowRegistry::new();
        let workflow = create_test_workflow("test_1");

        let result = registry.register(workflow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_duplicate_workflow() {
        let registry = WorkflowRegistry::new();
        let workflow = create_test_workflow("test_2");

        registry.register(workflow.clone()).await.unwrap();
        let result = registry.register(workflow).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_workflow() {
        let registry = WorkflowRegistry::new();
        let workflow = create_test_workflow("test_3");

        registry.register(workflow.clone()).await.unwrap();
        let retrieved = registry.get("test_3").await;
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test_3");
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let registry = WorkflowRegistry::new();
        
        registry.register(create_test_workflow("test_4")).await.unwrap();
        registry.register(create_test_workflow("test_5")).await.unwrap();

        let workflows = registry.list().await;
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_unregister_workflow() {
        let registry = WorkflowRegistry::new();
        
        registry.register(create_test_workflow("test_6")).await.unwrap();
        let result = registry.unregister("test_6").await;
        assert!(result.is_ok());

        let retrieved = registry.get("test_6").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_execution_lifecycle() {
        let registry = WorkflowRegistry::new();
        
        registry.register(create_test_workflow("test_7")).await.unwrap();
        
        let execution_id = registry.start_execution("test_7").await.unwrap();
        assert!(!execution_id.is_empty());

        let execution = registry.get_execution(&execution_id).await;
        assert!(execution.is_some());
        assert_eq!(execution.unwrap().status, WorkflowStatus::Running);

        let result = registry.execute_next_step(&execution_id).await;
        assert!(result.is_ok());

        let execution = registry.get_execution(&execution_id).await;
        assert_eq!(execution.unwrap().current_step, 1);
    }

    #[tokio::test]
    async fn test_cancel_execution() {
        let registry = WorkflowRegistry::new();
        
        registry.register(create_test_workflow("test_8")).await.unwrap();
        
        let execution_id = registry.start_execution("test_8").await.unwrap();
        let result = registry.cancel_execution(&execution_id).await;
        
        assert!(result.is_ok());
        
        let execution = registry.get_execution(&execution_id).await;
        assert_eq!(execution.unwrap().status, WorkflowStatus::Cancelled);
    }

    #[test]
    fn test_parse_workflow_from_yaml() {
        let yaml_content = r#"
id: test_workflow
name: Test Workflow
description: A test workflow
steps:
  - agent_id: ceo
    action: plan
  - agent_id: pm
    action: execute
"#;

        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(workflow.id, "test_workflow");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_extract_yaml_block() {
        let content = r#"
Some text before

```yaml
id: test
name: Test
```

Some text after
"#;

        let yaml = WorkflowRegistry::extract_yaml_block(content);
        assert!(yaml.is_some());
        assert!(yaml.unwrap().contains("id: test"));
    }
}
