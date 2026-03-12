//! Skill 服务 - 整合所有 Skill 相关功能
//!
//! 负责 Skills 的加载、热加载、应用配置等工作流

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use openagentic_agent::evo::registry::{DynamicSkill, SharedSkillRegistry};
use openagentic_agent::evo::skill_config_applier::{ConfigSkillApplier, ServerConfigSnapshot};
use openagentic_agent::evo::skill_hot_reloader::SkillHotReloader;
use openagentic_agent::evo::skill_loader::SkillLoader;
use openagentic_agent::evo::workflow_registry::WorkflowRegistry;

pub struct SkillService {
    skill_loader: Arc<SkillLoader>,
    hot_reloader: Arc<RwLock<Option<SkillHotReloader>>>,
    registry: Arc<SharedSkillRegistry>,
    workflow_registry: Arc<WorkflowRegistry>,
}

impl SkillService {
    pub fn new(registry: SharedSkillRegistry) -> Self {
        Self {
            skill_loader: Arc::new(SkillLoader::new(Arc::new(registry))),
            hot_reloader: Arc::new(RwLock::new(None)),
            registry: Arc::new(SharedSkillRegistry::new()),
            workflow_registry: Arc::new(WorkflowRegistry::new()),
        }
    }

    pub fn get_skill_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Ok(cwd) = std::env::current_dir() {
            let project_skills = cwd.join("skills");
            if project_skills.exists() {
                paths.push(project_skills);
            }
        }

        if let Some(home) = dirs::home_dir() {
            let user_skills = home.join(".open-agentic").join("skills");
            if user_skills.exists() {
                paths.push(user_skills);
            }
        }

        paths
    }

    pub async fn load_all_skills(&self) -> Result<Vec<DynamicSkill>, String> {
        let paths = Self::get_skill_paths();
        let mut all_skills = Vec::new();

        for path in paths {
            match self.skill_loader.load_from_directory(&path).await {
                Ok(skills) => {
                    tracing::info!("Loaded {} skills from {}", skills.len(), path.display());
                    all_skills.extend(skills);
                }
                Err(e) => {
                    tracing::warn!("Failed to load skills from {}: {}", path.display(), e);
                }
            }
        }

        for skill in &all_skills {
            self.registry.register_skill(skill.clone()).await;
        }

        tracing::info!("Total loaded {} skills", all_skills.len());
        Ok(all_skills)
    }

    pub async fn start_hot_reload(&self) -> Result<(), String> {
        let paths = Self::get_skill_paths();
        
        if paths.is_empty() {
            tracing::info!("No skill directories found, skipping hot reload");
            return Ok(());
        }

        let loader_for_reloader = SkillLoader::new(self.registry.clone_inner());
        let reloader = SkillHotReloader::new(loader_for_reloader);
        reloader.start(paths).await?;

        *self.hot_reloader.write().await = Some(reloader);
        
        tracing::info!("Skill hot reload started");
        Ok(())
    }

    pub async fn stop_hot_reload(&self) {
        if let Some(reloader) = self.hot_reloader.write().await.take() {
            reloader.stop().await;
            tracing::info!("Skill hot reload stopped");
        }
    }

    pub async fn apply_config_skills(&self, mut config: ServerConfigSnapshot) -> Result<ServerConfigSnapshot, String> {
        let skills: Vec<DynamicSkill> = self.registry.get_all_skills().await;
        
        for skill in skills {
            if skill.is_config() {
                config = ConfigSkillApplier::apply(&skill, config)?;
            }
        }

        Ok(config)
    }

    pub async fn load_workflow_skills(&self) -> Result<(), String> {
        let skills: Vec<DynamicSkill> = self.registry.get_all_skills().await;
        
        for skill in skills {
            if skill.is_workflow() {
                self.workflow_registry.apply_from_skill(&skill).await?;
            }
        }

        Ok(())
    }

    pub fn workflow_registry(&self) -> Arc<WorkflowRegistry> {
        self.workflow_registry.clone()
    }

    pub fn registry(&self) -> Arc<SharedSkillRegistry> {
        self.registry.clone_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_skill_paths() {
        let paths = SkillService::get_skill_paths();
        assert!(paths.iter().all(|p| p.ends_with("skills")));
    }

    #[tokio::test]
    async fn test_service_creation() {
        let registry = SharedSkillRegistry::new();
        let _service = SkillService::new(registry);
    }

    #[tokio::test]
    async fn test_apply_config_skills_empty() {
        let registry = SharedSkillRegistry::new();
        let service = SkillService::new(registry);
        
        let config = ServerConfigSnapshot::default();
        let result = service.apply_config_skills(config).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_registry_access() {
        let registry = SharedSkillRegistry::new();
        let service = SkillService::new(registry);
        
        let wf_registry = service.workflow_registry();
        let workflows = wf_registry.list().await;
        
        assert!(workflows.is_empty());
    }
}
