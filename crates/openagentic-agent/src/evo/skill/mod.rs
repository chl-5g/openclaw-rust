pub mod manager;
pub mod source;
pub mod local;

pub use manager::SkillManager;
pub use source::{Skill as ManagedSkill, SkillOrigin, InstalledSkill, SearchResult};
pub use local::LocalSkillManager;
