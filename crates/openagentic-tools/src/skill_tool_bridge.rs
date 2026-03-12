use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::skill_registry::SkillRegistry;
use crate::tool_registry::{Tool, ToolRegistry};

#[derive(Clone)]
pub struct SkillToolBridge {
    skill_registry: Arc<RwLock<SkillRegistry>>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
    skill_tool_map: Arc<RwLock<HashMap<String, String>>>,
}

impl SkillToolBridge {
    pub fn new() -> Self {
        Self {
            skill_registry: Arc::new(RwLock::new(SkillRegistry::new())),
            tool_registry: Arc::new(RwLock::new(ToolRegistry::new())),
            skill_tool_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_skill_registry(mut self, registry: SkillRegistry) -> Self {
        self.skill_registry = Arc::new(RwLock::new(registry));
        self
    }

    pub async fn register_skill_tool(
        &self,
        skill_id: &str,
        tool: Arc<dyn Tool>,
    ) -> Result<(), String> {
        let tool_name = tool.name().to_string();
        
        {
            let mut map = self.skill_tool_map.write().await;
            map.insert(skill_id.to_string(), tool_name.clone());
        }
        
        {
            let skill_registry = self.skill_registry.read().await;
            if skill_registry.get_all_skills().iter().any(|s| s.id == skill_id) {
                let mut tool_registry = self.tool_registry.write().await;
                tool_registry.register(tool_name, tool);
            }
        }
        
        Ok(())
    }

    pub async fn enable_skill(&self, skill_id: &str) -> Result<(), String> {
        {
            let mut registry = self.skill_registry.write().await;
            registry.enable_skill(skill_id)?;
        }
        
        let tool_name = {
            let map = self.skill_tool_map.read().await;
            map.get(skill_id).cloned()
        };

        if let Some(ref tool_name) = tool_name {
            let skill_registry = self.skill_registry.read().await;
            if skill_registry.get_all_skills().iter().any(|s| s.id == skill_id && s.enabled) {
                let tool_registry = self.tool_registry.read().await;
                if !tool_registry.has_tool(tool_name) {
                    return Err(format!("Tool for skill {} not registered", skill_id));
                }
            }
        }

        Ok(())
    }

    pub async fn disable_skill(&self, skill_id: &str) -> Result<(), String> {
        let mut registry = self.skill_registry.write().await;
        registry.disable_skill(skill_id)?;
        Ok(())
    }

    pub async fn get_skill_registry(&self) -> Arc<RwLock<SkillRegistry>> {
        self.skill_registry.clone()
    }

    pub async fn get_tool_registry(&self) -> Arc<RwLock<ToolRegistry>> {
        self.tool_registry.clone()
    }

    pub async fn get_enabled_tools(&self) -> Vec<String> {
        let skill_registry = self.skill_registry.read().await;
        let map = self.skill_tool_map.read().await;
        let tool_registry = self.tool_registry.read().await;

        skill_registry
            .get_enabled_skills()
            .iter()
            .filter_map(|s| map.get(&s.id))
            .filter(|tool_name| tool_registry.has_tool(tool_name))
            .cloned()
            .collect()
    }

    pub async fn sync_to_tool_registry(&self) {
        let skill_registry = self.skill_registry.read().await;
        let map = self.skill_tool_map.read().await;
        
        let enabled_skill_ids: Vec<String> = skill_registry
            .get_enabled_skills()
            .iter()
            .map(|s| s.id.clone())
            .collect();
        
        drop(skill_registry);

        let tool_registry = self.tool_registry.read().await;
        for skill_id in enabled_skill_ids {
            if let Some(tool_name) = map.get(&skill_id) {
                if !tool_registry.has_tool(tool_name) {
                    tracing::warn!("Skill {} enabled but tool {} not registered", skill_id, tool_name);
                }
            }
        }
    }
}

impl Default for SkillToolBridge {
    fn default() -> Self {
        Self::new()
    }
}
