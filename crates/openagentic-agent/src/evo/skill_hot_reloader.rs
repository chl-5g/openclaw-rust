//! Skill 热加载器
//!
//! 监控 Skills 目录变化，支持动态添加、删除、修改 Skills

use std::path::PathBuf;
use std::sync::Arc;

use openagentic_memory::file_watcher::{FileChange, FileChangeType, FileWatcher, FileWatcherConfig};
use tokio::sync::RwLock;

use super::registry::DynamicSkill;
use super::skill_loader::SkillLoader;

pub type SkillChangeCallback = Arc<dyn Fn(SkillChangeEvent) + Send + Sync>;

#[derive(Debug, Clone)]
pub enum SkillChangeEvent {
    Created(DynamicSkill),
    Updated(DynamicSkill),
    Removed(String),
}

pub struct SkillHotReloader {
    skill_loader: Arc<SkillLoader>,
    watcher: Arc<RwLock<Option<FileWatcher>>>,
    skills: Arc<RwLock<Vec<DynamicSkill>>>,
    callback: Option<SkillChangeCallback>,
}

impl SkillHotReloader {
    pub fn new(skill_loader: SkillLoader) -> Self {
        Self {
            skill_loader: Arc::new(skill_loader),
            watcher: Arc::new(RwLock::new(None)),
            skills: Arc::new(RwLock::new(Vec::new())),
            callback: None,
        }
    }

    pub fn with_callback(mut self, callback: SkillChangeCallback) -> Self {
        self.callback = Some(callback);
        self
    }

    pub async fn start(&self, skill_paths: Vec<PathBuf>) -> Result<(), String> {
        // 初始加载所有 Skills
        self.reload_skills(skill_paths.clone()).await?;

        // 配置并启动文件监控
        let mut config = FileWatcherConfig::default();
        config.watch_paths = skill_paths;
        config.poll_interval_ms = 2000;
        config.ignored_patterns = vec![
            "*.tmp".to_string(),
            "*.swp".to_string(),
            ".git".to_string(),
            "*.swp".to_string(),
        ];

        let skill_loader = self.skill_loader.clone();
        let skills = self.skills.clone();
        let callback = self.callback.clone();

        let watcher = FileWatcher::new(config)
            .with_callback(Arc::new(move |change| {
                let skill_loader = skill_loader.clone();
                let skills = skills.clone();
                let callback = callback.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_file_change(change, &skill_loader, &skills, &callback).await {
                        tracing::error!("Error handling file change: {}", e);
                    }
                });
            }));

        watcher.start().await?;

        *self.watcher.write().await = Some(watcher);
        
        tracing::info!("Skill hot reloader started");
        Ok(())
    }

    async fn reload_skills(&self, skill_paths: Vec<PathBuf>) -> Result<(), String> {
        let mut all_skills = Vec::new();

        for path in skill_paths {
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

        *self.skills.write().await = all_skills.clone();
        
        // 触发回调
        if let Some(callback) = &self.callback {
            for skill in all_skills {
                callback(SkillChangeEvent::Created(skill));
            }
        }

        Ok(())
    }

    async fn handle_file_change(
        change: FileChange,
        skill_loader: &Arc<SkillLoader>,
        skills: &Arc<RwLock<Vec<DynamicSkill>>>,
        callback: &Option<SkillChangeCallback>,
    ) -> Result<(), String> {
        let path = &change.path;

        // 检查是否是 SKILL.md 文件
        if !path.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
            return Ok(());
        }

        // 获取技能目录
        let skill_dir = path.parent().ok_or("Invalid skill path")?;

        match change.change_type {
            FileChangeType::Created | FileChangeType::Modified => {
                // 重新加载该 Skill
                match skill_loader.load_from_file(path).await {
                    Ok(mut skill) => {
                        // 添加目录路径到 metadata
                        skill.metadata.insert("skill_dir".into(), skill_dir.to_string_lossy().into());
                        
                        let references_dir = skill_dir.join("references");
                        if references_dir.exists() {
                            skill.metadata.insert("references_dir".into(), references_dir.to_string_lossy().into());
                        }
                        let scripts_dir = skill_dir.join("scripts");
                        if scripts_dir.exists() {
                            skill.metadata.insert("scripts_dir".into(), scripts_dir.to_string_lossy().into());
                        }
                        let assets_dir = skill_dir.join("assets");
                        if assets_dir.exists() {
                            skill.metadata.insert("assets_dir".into(), assets_dir.to_string_lossy().into());
                        }

                        // 更新内存中的 skills
                        let skill_id = skill.id.clone();
                        let mut skills_guard = skills.write().await;
                        
                        if let Some(pos) = skills_guard.iter().position(|s| s.id == skill_id) {
                            skills_guard[pos] = skill.clone();
                            if let Some(cb) = callback {
                                cb(SkillChangeEvent::Updated(skill));
                            }
                        } else {
                            skills_guard.push(skill.clone());
                            if let Some(cb) = callback {
                                cb(SkillChangeEvent::Created(skill));
                            }
                        }

                        tracing::info!("Reloaded skill: {}", skill_id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to reload skill from {}: {}", path.display(), e);
                    }
                }
            }
            FileChangeType::Removed => {
                // 从内存中移除
                let skill_dir_name = skill_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                let mut skills_guard = skills.write().await;
                if let Some(pos) = skills_guard.iter().position(|s| {
                    s.metadata.get("skill_dir")
                        .map(|d| d.contains(skill_dir_name))
                        .unwrap_or(false)
                }) {
                    let removed = skills_guard.remove(pos);
                    let removed_id = removed.id.clone();
                    let log_id = removed_id.clone();
                    if let Some(cb) = callback {
                        cb(SkillChangeEvent::Removed(removed_id));
                    }
                    tracing::info!("Removed skill: {}", log_id);
                }
            }
        }

        Ok(())
    }

    pub async fn get_skills(&self) -> Vec<DynamicSkill> {
        self.skills.read().await.clone()
    }

    pub async fn get_skill(&self, id: &str) -> Option<DynamicSkill> {
        self.skills.read().await.iter().find(|s| s.id == id).cloned()
    }

    pub async fn stop(&self) {
        if let Some(watcher) = self.watcher.write().await.take() {
            if let Err(e) = watcher.stop().await {
                tracing::warn!("Error stopping file watcher: {}", e);
            }
            tracing::info!("Skill hot reloader stopped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evo::registry::SharedSkillRegistry;

    #[tokio::test]
    async fn test_skill_hot_reloader() {
        let registry = Arc::new(SharedSkillRegistry::default());
        let skill_loader = SkillLoader::new(registry);
        let reloader = SkillHotReloader::new(skill_loader);
        
        assert!(reloader.get_skills().await.is_empty());
    }
}
