//! Config Skill 应用器
//!
//! 用于解析 Config Skill 并将其应用到 ServerConfig

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::registry::DynamicSkill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSkillData {
    #[serde(default)]
    pub agents: Vec<AgentConfigEntry>,
    #[serde(default)]
    pub channels: Vec<ChannelConfigEntry>,
    #[serde(default)]
    pub tools: Vec<ToolConfigEntry>,
    #[serde(default)]
    pub bindings: Vec<BindingConfigEntry>,
    #[serde(default)]
    pub agent_to_agent: Option<AgentToAgentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfigEntry {
    #[serde(rename = "type")]
    pub channel_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub default: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfigEntry {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfigEntry {
    pub agent_id: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToAgentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub delegation_mode: Option<String>,
    #[serde(default)]
    pub rules: Vec<AgentToAgentRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToAgentRule {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default = "default_max_hops")]
    pub max_hops: u32,
}

fn default_max_hops() -> u32 {
    3
}

pub struct ConfigSkillApplier;

impl ConfigSkillApplier {
    pub fn apply(skill: &DynamicSkill, config: ServerConfigSnapshot) -> Result<ServerConfigSnapshot, String> {
        if !skill.is_config() {
            return Err("Skill is not a config type".to_string());
        }

        let config_data = Self::parse_config_from_skill(skill)?;

        let mut new_config = config;

        // 应用 agents 配置
        if !config_data.agents.is_empty() {
            new_config = Self::apply_agents_config(config_data.agents, new_config);
        }

        // 应用 channels 配置
        if !config_data.channels.is_empty() {
            new_config = Self::apply_channels_config(config_data.channels, new_config);
        }

        // 应用 tools 配置
        if !config_data.tools.is_empty() {
            new_config = Self::apply_tools_config(config_data.tools, new_config);
        }

        // 应用 bindings 配置
        if !config_data.bindings.is_empty() {
            new_config = Self::apply_bindings_config(config_data.bindings, new_config);
        }

        // 应用 agent_to_agent 配置
        if let Some(ata_config) = config_data.agent_to_agent {
            new_config = Self::apply_agent_to_agent_config(ata_config, new_config);
        }

        tracing::info!("Applied config skill: {}", skill.name);
        Ok(new_config)
    }

    fn parse_config_from_skill(skill: &DynamicSkill) -> Result<ConfigSkillData, String> {
        // 从 skill.instructions 中解析 YAML 配置
        // 查找 ```yaml ... ``` 代码块
        if let Some(instructions) = &skill.instructions {
            if let Some(config_yaml) = Self::extract_yaml_block(instructions) {
                return serde_yaml::from_str(&config_yaml)
                    .map_err(|e| format!("Failed to parse config: {}", e));
            }
        }

        // 如果没有找到 YAML 块，返回空配置
        Ok(ConfigSkillData::default())
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

    fn apply_agents_config(agents: Vec<AgentConfigEntry>, mut config: ServerConfigSnapshot) -> ServerConfigSnapshot {
        for agent in agents {
            config.agents.push(AgentSnapshot {
                id: agent.id,
                name: agent.name,
                role: agent.role.unwrap_or_default(),
                description: agent.description.unwrap_or_default(),
                capabilities: agent.capabilities,
            });
        }
        config
    }

    fn apply_channels_config(channels: Vec<ChannelConfigEntry>, mut config: ServerConfigSnapshot) -> ServerConfigSnapshot {
        for channel in channels {
            config.channel_settings.insert(
                channel.channel_type.clone(),
                ChannelSettingSnapshot {
                    enabled: channel.enabled,
                    is_default: channel.default.unwrap_or(false),
                },
            );
        }
        config
    }

    fn apply_tools_config(tools: Vec<ToolConfigEntry>, mut config: ServerConfigSnapshot) -> ServerConfigSnapshot {
        for tool in tools {
            config.tools.push(ToolSnapshot {
                name: tool.name,
                enabled: tool.enabled,
                description: tool.description.unwrap_or_default(),
            });
        }
        config
    }

    fn apply_bindings_config(bindings: Vec<BindingConfigEntry>, mut config: ServerConfigSnapshot) -> ServerConfigSnapshot {
        for binding in bindings {
            config.bindings.push(BindingSnapshot {
                agent_id: binding.agent_id,
                channels: binding.channels,
                tools: binding.tools,
            });
        }
        config
    }

    fn apply_agent_to_agent_config(ata: AgentToAgentConfig, mut config: ServerConfigSnapshot) -> ServerConfigSnapshot {
        config.agent_to_agent = Some(AgentToAgentSnapshot {
            enabled: ata.enabled,
            delegation_mode: ata.delegation_mode.unwrap_or_else(|| "explicit".to_string()),
            rules: ata.rules.into_iter().map(|r| AgentToAgentRuleSnapshot {
                from: r.from,
                to: r.to,
                conditions: r.conditions,
                max_hops: r.max_hops,
            }).collect(),
        });
        config
    }

    pub fn get_skill_dir(skill: &DynamicSkill) -> Option<PathBuf> {
        skill.metadata.get("skill_dir")
            .map(PathBuf::from)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerConfigSnapshot {
    #[serde(default)]
    pub agents: Vec<AgentSnapshot>,
    #[serde(default)]
    pub channel_settings: std::collections::HashMap<String, ChannelSettingSnapshot>,
    #[serde(default)]
    pub tools: Vec<ToolSnapshot>,
    #[serde(default)]
    pub bindings: Vec<BindingSnapshot>,
    #[serde(default)]
    pub agent_to_agent: Option<AgentToAgentSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettingSnapshot {
    pub enabled: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSnapshot {
    pub name: String,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingSnapshot {
    pub agent_id: String,
    pub channels: Vec<String>,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToAgentSnapshot {
    pub enabled: bool,
    pub delegation_mode: String,
    pub rules: Vec<AgentToAgentRuleSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToAgentRuleSnapshot {
    pub from: String,
    pub to: String,
    pub conditions: Vec<String>,
    pub max_hops: u32,
}

impl Default for ConfigSkillData {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            channels: Vec::new(),
            tools: Vec::new(),
            bindings: Vec::new(),
            agent_to_agent: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill(name: &str, skill_type: super::super::registry::SkillType) -> DynamicSkill {
        use super::super::registry::{SkillFormat, SkillSource};
        use chrono::Utc;
        use std::collections::HashMap;

        DynamicSkill {
            id: format!("skill_{}", name),
            name: name.to_string(),
            description: "Test skill".to_string(),
            format: SkillFormat::AgentSkills,
            skill_type,
            code: None,
            instructions: None,
            language: "prompt".to_string(),
            source: SkillSource::default(),
            gating: None,
            compatibility: None,
            metadata: HashMap::new(),
            allowed_tools: Vec::new(),
            created_by: "test".to_string(),
            created_at: Utc::now(),
            version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn test_apply_empty_config() {
        let skill = create_test_skill("test", super::super::registry::SkillType::Config);
        let config = ServerConfigSnapshot::default();
        
        let result = ConfigSkillApplier::apply(&skill, config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_non_config_skill_fails() {
        let skill = create_test_skill("test", super::super::registry::SkillType::Prompt);
        let config = ServerConfigSnapshot::default();
        
        let result = ConfigSkillApplier::apply(&skill, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_yaml_block() {
        let content = r#"
Some text before

```yaml
agents:
  - id: ceo
    name: CEO Agent
```

Some text after
"#;

        let yaml = ConfigSkillApplier::extract_yaml_block(content);
        assert!(yaml.is_some());
        let yaml = yaml.unwrap();
        assert!(yaml.contains("agents:"));
    }

    #[test]
    fn test_apply_agents_config() {
        let agents = vec![
            AgentConfigEntry {
                id: "ceo".to_string(),
                name: "CEO Agent".to_string(),
                role: Some("executive".to_string()),
                description: Some("CEO description".to_string()),
                capabilities: vec!["planning".to_string()],
            },
        ];

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply_agents_config(agents, config);
        
        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].id, "ceo");
    }

    #[test]
    fn test_apply_channels_config() {
        let channels = vec![
            ChannelConfigEntry {
                channel_type: "feishu".to_string(),
                enabled: true,
                default: Some(true),
            },
        ];

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply_channels_config(channels, config);
        
        assert!(result.channel_settings.contains_key("feishu"));
    }

    #[test]
    fn test_apply_tools_config() {
        let tools = vec![
            ToolConfigEntry {
                name: "browser".to_string(),
                enabled: true,
                description: Some("Browser tool".to_string()),
            },
        ];

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply_tools_config(tools, config);
        
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "browser");
    }

    #[test]
    fn test_apply_bindings_config() {
        let bindings = vec![
            BindingConfigEntry {
                agent_id: "ceo".to_string(),
                channels: vec!["feishu".to_string()],
                tools: vec!["browser".to_string()],
            },
        ];

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply_bindings_config(bindings, config);
        
        assert_eq!(result.bindings.len(), 1);
        assert_eq!(result.bindings[0].agent_id, "ceo");
    }

    #[test]
    fn test_apply_agent_to_agent_config() {
        let ata = AgentToAgentConfig {
            enabled: true,
            delegation_mode: Some("explicit".to_string()),
            rules: vec![
                AgentToAgentRule {
                    from: "ceo".to_string(),
                    to: "pm".to_string(),
                    conditions: vec!["task_type == product".to_string()],
                    max_hops: 2,
                },
            ],
        };

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply_agent_to_agent_config(ata, config);
        
        assert!(result.agent_to_agent.is_some());
        assert!(result.agent_to_agent.unwrap().enabled);
    }

    #[test]
    fn test_full_config_apply() {
        use super::super::registry::{SkillFormat, SkillSource};
        use chrono::Utc;
        use std::collections::HashMap;

        let content = r#"
```yaml
agents:
  - id: ceo
    name: CEO Agent
    role: executive
  - id: pm
    name: Product Manager
    role: manager

channels:
  - type: feishu
    enabled: true
  - type: discord
    enabled: false

tools:
  - name: browser
    enabled: true

bindings:
  - agent_id: ceo
    channels:
      - feishu
    tools:
      - browser

agent_to_agent:
  enabled: true
  delegation_mode: explicit
  rules:
    - from: ceo
      to: pm
      conditions:
        - task_type == product
```
"#;

        let skill = DynamicSkill {
            id: "skill_full_config".to_string(),
            name: "Full Config".to_string(),
            description: "Full config test".to_string(),
            format: SkillFormat::AgentSkills,
            skill_type: super::super::registry::SkillType::Config,
            code: None,
            instructions: Some(content.to_string()),
            language: "prompt".to_string(),
            source: SkillSource::default(),
            gating: None,
            compatibility: None,
            metadata: HashMap::new(),
            allowed_tools: Vec::new(),
            created_by: "test".to_string(),
            created_at: Utc::now(),
            version: "1.0.0".to_string(),
        };

        let config = ServerConfigSnapshot::default();
        let result = ConfigSkillApplier::apply(&skill, config);
        
        assert!(result.is_ok());
        let result = result.unwrap();
        
        assert_eq!(result.agents.len(), 2);
        assert_eq!(result.channel_settings.len(), 2);
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.bindings.len(), 1);
        assert!(result.agent_to_agent.is_some());
    }
}
