use std::path::Path;
use std::sync::Arc;

use crate::evo::registry::{DynamicSkill, SharedSkillRegistry, SkillSource};
use crate::evo::skill_loader::SkillLoader;
use crate::evo::propagation::SkillPropagationService;
use crate::evo::{DynamicCompiler, ProgrammingLanguage};

pub struct UnifiedSkillService {
    registry: Arc<SharedSkillRegistry>,
    loader: SkillLoader,
    compiler: DynamicCompiler,
    propagation: Option<Arc<SkillPropagationService>>,
}

impl UnifiedSkillService {
    pub fn new(registry: Arc<SharedSkillRegistry>) -> Self {
        Self {
            registry: registry.clone(),
            loader: SkillLoader::new(registry.clone()),
            compiler: DynamicCompiler::new(ProgrammingLanguage::Wasm),
            propagation: None,
        }
    }

    pub fn with_propagation(mut self, propagation: Arc<SkillPropagationService>) -> Self {
        self.propagation = Some(propagation);
        self
    }

    pub fn registry(&self) -> Arc<SharedSkillRegistry> {
        self.registry.clone_inner()
    }

    pub async fn load_user_skills(&self, dir: &Path) -> Result<Vec<DynamicSkill>, String> {
        let skills = self.loader.load_from_directory(dir).await?;
        
        for skill in &skills {
            let mut user_skill = skill.clone();
            user_skill.source = SkillSource::User;
            self.registry.register_skill(user_skill).await;
        }
        
        Ok(skills)
    }

    pub async fn register_evo_skill(&self, skill: DynamicSkill) -> Result<(), String> {
        let mut evo_skill = skill;
        evo_skill.source = SkillSource::Evo;
        
        self.registry.register_skill(evo_skill.clone()).await;
        
        if let Some(ref propagation) = self.propagation {
            propagation.propagate_skill(evo_skill).await?;
        }
        
        Ok(())
    }

    pub async fn get_all_skills(&self) -> Vec<DynamicSkill> {
        self.registry.get_all_skills().await
    }

    pub async fn get_skills_by_source(&self, source: SkillSource) -> Vec<DynamicSkill> {
        self.registry.get_skills_by_source(source).await
    }

    pub async fn get_skill(&self, name: &str) -> Option<DynamicSkill> {
        self.registry.get_skill_by_name(name).await
    }
}
