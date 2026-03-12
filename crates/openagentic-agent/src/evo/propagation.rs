use std::path::Path;
use std::sync::Arc;

use openagentic_acp::router::Router;

use crate::evo::registry::{DynamicSkill, SharedSkillRegistry, SkillSource};

pub struct SkillPropagationService {
    router: Arc<Router>,
    registry: Arc<SharedSkillRegistry>,
}

impl SkillPropagationService {
    pub fn new(router: Arc<Router>, registry: Arc<SharedSkillRegistry>) -> Self {
        Self { router, registry }
    }

    pub async fn propagate_skill(&self, skill: DynamicSkill) -> Result<(), String> {
        let agent_ids = self.router.broadcast("skill_propagation").await;
        
        for agent_id in agent_ids {
            self.registry.register_skill(skill.clone()).await;
        }
        
        Ok(())
    }

    pub async fn propagate_skills(&self, skills: Vec<DynamicSkill>) -> Result<(), String> {
        for skill in skills {
            self.propagate_skill(skill).await?;
        }
        Ok(())
    }
}
