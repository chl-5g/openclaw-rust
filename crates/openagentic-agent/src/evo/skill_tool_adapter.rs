use std::sync::Arc;

use async_trait::async_trait;
use openagentic_core::{OpenAgenticError, Result};

use crate::evo::registry::SharedSkillRegistry;
use crate::ports::{ToolInfo, ToolPort};

pub struct SkillToolAdapter {
    registry: Arc<SharedSkillRegistry>,
}

impl SkillToolAdapter {
    pub fn new(registry: Arc<SharedSkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolPort for SkillToolAdapter {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let skill = self.registry
            .get_skill_by_name(tool_name)
            .await
            .ok_or_else(|| OpenAgenticError::Tool(format!("Skill not found: {}", tool_name)))?;

        let compiled = self.registry
            .get_compiled_skill(&skill.id)
            .await
            .ok_or_else(|| OpenAgenticError::Tool(format!("Compiled skill not found: {}", skill.id)))?;

        Ok(serde_json::json!({
            "status": "success",
            "skill": skill.name,
            "compiled_at": compiled.compiled_at.to_rfc3339(),
            "arguments": arguments,
        }))
    }

    async fn execute_with_sandbox(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        enable_sandbox: bool,
    ) -> Result<serde_json::Value> {
        if enable_sandbox {
            self.execute(tool_name, arguments).await
        } else {
            Err(OpenAgenticError::Execution("Sandbox disabled".to_string()))
        }
    }

    async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let skills = self.registry.get_all_skills().await;
        
        Ok(skills.into_iter().map(|s| ToolInfo {
            name: s.name.clone(),
            description: format!("User-defined skill: {}", s.name),
            parameters: serde_json::json!({}),
        }).collect())
    }
}
