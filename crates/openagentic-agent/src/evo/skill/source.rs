use serde::{Deserialize, Serialize};

use crate::evo::registry::{DynamicSkill, SkillFormat, SkillSource};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillOrigin {
    Local,
    ClawHub,
    Bundled,
}

impl Default for SkillOrigin {
    fn default() -> Self {
        SkillOrigin::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub origin: SkillOrigin,
    pub format: SkillFormat,
    pub dynamic_skill: DynamicSkill,
}

impl Skill {
    pub fn new(dynamic_skill: DynamicSkill, origin: SkillOrigin) -> Self {
        Self {
            id: dynamic_skill.id.clone(),
            name: dynamic_skill.name.clone(),
            version: dynamic_skill.version.clone(),
            description: dynamic_skill.description.clone(),
            origin,
            format: dynamic_skill.format.clone(),
            dynamic_skill,
        }
    }

    pub fn from_local(dynamic_skill: DynamicSkill) -> Self {
        Self::new(dynamic_skill, SkillOrigin::Local)
    }

    pub fn from_hub(dynamic_skill: DynamicSkill) -> Self {
        Self::new(dynamic_skill, SkillOrigin::ClawHub)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub name: String,
    pub version: String,
    pub origin: SkillOrigin,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub format: SkillFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub score: f32,
    pub downloads: u64,
    pub rating: f32,
}
