//! 通道配置 CLI 工具
//!
//! 提供命令行接口来管理各通道的配置

use clap::Subcommand;
use openagentic_core::OpenAgenticError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 通道配置文件路径
fn get_channel_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".open-agentic").join("channels.json")
}

/// 通道配置管理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfigManager {
    /// 各通道配置
    pub channels: HashMap<String, ChannelConfig>,
    /// 默认通道
    #[serde(default)]
    pub default_channel: Option<String>,
}

/// 单个通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 通道类型
    pub channel_type: String,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 配置参数
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

impl ChannelConfigManager {
    /// 加载配置
    pub fn load() -> Result<Self, OpenAgenticError> {
        let path = get_channel_config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| OpenAgenticError::Config(format!("读取通道配置失败: {}", e)))?;
            serde_json::from_str(&content).map_err(OpenAgenticError::Serialization)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), OpenAgenticError> {
        let path = get_channel_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OpenAgenticError::Config(format!("创建配置目录失败: {}", e)))?;
        }
        let content = serde_json::to_string_pretty(self).map_err(OpenAgenticError::Serialization)?;
        std::fs::write(&path, content)
            .map_err(|e| OpenAgenticError::Config(format!("保存通道配置失败: {}", e)))?;
        Ok(())
    }

    /// 获取通道配置
    pub fn get_channel(&self, name: &str) -> Option<&ChannelConfig> {
        self.channels.get(name)
    }

    /// 设置通道配置
    pub fn set_channel(&mut self, name: String, config: ChannelConfig) {
        self.channels.insert(name, config);
    }

    /// 删除通道配置
    pub fn remove_channel(&mut self, name: &str) -> bool {
        self.channels.remove(name).is_some()
    }

    /// 列出所有通道
    pub fn list_channels(&self) -> Vec<&String> {
        self.channels.keys().collect()
    }
}

/// 支持的通道类型
const CHANNEL_TYPES: &[(&str, &str)] = &[
    ("dingtalk", "钉钉"),
    ("wecom", "企业微信"),
    ("feishu", "飞书"),
    ("discord", "Discord"),
    ("teams", "Microsoft Teams"),
    ("slack", "Slack"),
    ("whatsapp", "WhatsApp"),
    ("telegram", "Telegram"),
];

#[derive(Debug, Subcommand)]
pub enum ChannelCommand {
    /// 列出所有可用通道
    List,
    /// 登录通道 (显示设置说明)
    Login {
        /// 通道名称
        #[arg(default_value = "")]
        channel: String,
    },
    /// 显示通道状态
    Status,
    /// 设置通道配置
    Set {
        /// 通道类型 (dingtalk, wecom, feishu, discord, teams, slack, whatsapp, telegram)
        channel_type: String,
        /// 配置参数 (格式: key=value，可多次使用)
        #[arg(short, long = "config", value_parser = parse_key_value, action = clap::ArgAction::Append)]
        configs: Vec<(String, String)>,
        /// 启用通道
        #[arg(short, long)]
        enable: bool,
    },

    /// 获取通道配置
    Get {
        /// 通道类型
        channel_type: String,
    },

    /// 删除通道配置
    Remove {
        /// 通道类型
        channel_type: String,
    },

    /// 启用通道
    Enable {
        /// 通道类型
        channel_type: String,
    },

    /// 禁用通道
    Disable {
        /// 通道类型
        channel_type: String,
    },

    /// 设置默认通道
    Default {
        /// 通道类型
        channel_type: String,
    },

    /// 测试通道连接
    Test {
        /// 通道类型
        channel_type: String,
        /// 测试消息
        #[arg(short, long, default_value = "测试消息")]
        message: String,
        /// 目标 ID (如 chat_id, phone number)
        #[arg(short, long)]
        target: Option<String>,
    },
}

/// 解析 key=value 格式
fn parse_key_value(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("无效的配置格式: {}，应为 key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

impl ChannelCommand {
    /// 执行命令
    pub async fn execute(&self) -> Result<(), OpenAgenticError> {
        let mut manager = ChannelConfigManager::load()?;

        match self {
            ChannelCommand::List => {
                println!();
                println!("\x1b[36m\x1b[1m📱 Available Chat Channels\x1b[0m");
                println!();

                let channels = vec![
                    ("telegram", "Telegram", "Bot Token"),
                    ("discord", "Discord", "Bot Token"),
                    ("whatsapp", "WhatsApp", "QR Code"),
                    ("feishu", "飞书 (Feishu)", "App ID/Secret"),
                    ("dingtalk", "钉钉 (DingTalk)", "App Key/Secret"),
                    ("wecom", "企业微信 (WeCom)", "Corp ID/Agent ID"),
                    ("slack", "Slack", "Bot Token"),
                ];

                for (name, display, auth) in channels {
                    println!("  \x1b[33m{}\x1b[0m", name);
                    println!("    \x1b[90m{} | Auth: {}\x1b[0m", display, auth);
                }

                println!();
                Ok(())
            }

            ChannelCommand::Login { channel } => {
                if channel.is_empty() {
                    println!();
                    println!("\x1b[36m\x1b[1m📱 Available Chat Channels\x1b[0m");
                    println!();
                    println!("Usage: \x1b[36mopen-agentic channel login <channel-name>\x1b[0m");
                    println!();
                    println!("Supported channels:");
                    println!("  whatsapp  - WhatsApp (QR Code)");
                    println!("  telegram   - Telegram (Bot)");
                    println!("  discord    - Discord (Bot)");
                } else {
                    match channel.to_lowercase().as_str() {
                        "whatsapp" => {
                            println!();
                            println!("\x1b[33m📱 WhatsApp Login\x1b[0m");
                            println!();
                            println!("  Run: \x1b[36mopen-agentic channel login whatsapp\x1b[0m");
                            println!("  Then scan QR code with your phone");
                        }
                        "telegram" => {
                            println!();
                            println!("\x1b[33m📱 Telegram Setup\x1b[0m");
                            println!();
                            println!("  1. Search @BotFather in Telegram");
                            println!("  2. Send /newbot to create bot");
                            println!(
                                "  3. Run: \x1b[36mopen-agentic channel set telegram --config bot_token=YOUR_TOKEN --enable\x1b[0m"
                            );
                        }
                        "discord" => {
                            println!();
                            println!("\x1b[33m📱 Discord Setup\x1b[0m");
                            println!();
                            println!("  1. Go to Discord Developer Portal");
                            println!("  2. Create app and add bot");
                            println!(
                                "  3. Run: \x1b[36mopen-agentic channel set discord --config bot_token=YOUR_TOKEN --enable\x1b[0m"
                            );
                        }
                        _ => {
                            println!("\x1b[31mUnknown channel: {}\x1b[0m", channel);
                        }
                    }
                }
                Ok(())
            }

            ChannelCommand::Status => {
                println!();
                println!("\x1b[36m\x1b[1m📡 Channel Status\x1b[0m");
                println!();

                let channels = vec![
                    ("telegram", "Telegram"),
                    ("discord", "Discord"),
                    ("whatsapp", "WhatsApp"),
                    ("feishu", "飞书"),
                    ("dingtalk", "钉钉"),
                    ("wecom", "企业微信"),
                    ("slack", "Slack"),
                ];

                for (name, display) in channels {
                    let config = manager.get_channel(name);
                    let enabled = config.map(|c| c.enabled).unwrap_or(false);
                    let status = if enabled {
                        "\x1b[32m✓ Enabled\x1b[0m"
                    } else {
                        "\x1b[90m○ Disabled\x1b[0m"
                    };
                    println!("  \x1b[33m{}\x1b[0m  {}", display, status);
                }

                Ok(())
            }

            ChannelCommand::Set {
                channel_type,
                configs,
                enable,
            } => {
                let channel_type_lower = channel_type.to_lowercase();

                if !CHANNEL_TYPES.iter().any(|(t, _)| *t == channel_type_lower) {
                    println!("❌ 不支持的通道类型: {}", channel_type);
                    println!("\n支持的通道类型:");
                    for (t, name) in CHANNEL_TYPES {
                        println!("  {} - {}", t, name);
                    }
                    return Ok(());
                }

                let mut config_map = HashMap::new();
                for (key, value) in configs {
                    let json_value = if value.starts_with('"') && value.ends_with('"') {
                        serde_json::Value::String(value[1..value.len() - 1].to_string())
                    } else if value == "true" || value == "false" {
                        serde_json::Value::Bool(value == "true")
                    } else if let Ok(n) = value.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else {
                        serde_json::Value::String(value.clone())
                    };
                    config_map.insert(key.clone(), json_value);
                }

                let config = ChannelConfig {
                    channel_type: channel_type_lower.clone(),
                    enabled: *enable,
                    config: config_map,
                };

                manager.set_channel(channel_type_lower.clone(), config);
                manager.save()?;

                println!("✅ 成功设置 {} 通道配置", channel_type);
                if *enable {
                    println!("   状态: 已启用");
                }
                println!(
                    "\n使用 'open-agentic channel test {}' 测试连接",
                    channel_type
                );
                Ok(())
            }

            ChannelCommand::Get { channel_type } => {
                if let Some(config) = manager.get_channel(channel_type) {
                    println!("通道: {} ({})", channel_type, config.channel_type);
                    println!(
                        "状态: {}",
                        if config.enabled {
                            "已启用"
                        } else {
                            "已禁用"
                        }
                    );
                    println!("\n配置:");
                    for (key, value) in &config.config {
                        if key.contains("token") || key.contains("secret") || key.contains("key") {
                            let masked = mask_sensitive_value(value);
                            println!("  {}: {}", key, masked);
                        } else {
                            println!("  {}: {}", key, value);
                        }
                    }
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                    println!(
                        "\n使用 'open-agentic channel set {}' 创建配置",
                        channel_type
                    );
                }
                Ok(())
            }

            ChannelCommand::Remove { channel_type } => {
                if manager.remove_channel(channel_type) {
                    manager.save()?;
                    println!("✅ 已删除 {} 通道配置", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
                Ok(())
            }

            ChannelCommand::Enable { channel_type } => {
                if let Some(config) = manager.channels.get_mut(channel_type) {
                    config.enabled = true;
                    manager.save()?;
                    println!("✅ 已启用 {} 通道", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
                Ok(())
            }

            ChannelCommand::Disable { channel_type } => {
                if let Some(config) = manager.channels.get_mut(channel_type) {
                    config.enabled = false;
                    manager.save()?;
                    println!("✅ 已禁用 {} 通道", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
                Ok(())
            }

            ChannelCommand::Default { channel_type } => {
                if manager.get_channel(channel_type).is_some() {
                    manager.default_channel = Some(channel_type.clone());
                    manager.save()?;
                    println!("✅ 已设置默认通道: {}", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
                Ok(())
            }

            ChannelCommand::Test {
                channel_type,
                message,
                target,
            } => {
                println!("🔍 测试 {} 通道...", channel_type);

                if let Some(_config) = manager.get_channel(channel_type) {
                    println!("   消息: {}", message);
                    if let Some(t) = target {
                        println!("   目标: {}", t);
                    }
                    println!("\n⚠️  测试功能开发中，请手动验证配置");
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
                Ok(())
            }
        }
    }
}

fn mask_sensitive_value(value: &serde_json::Value) -> String {
    let s = value.as_str().unwrap_or("");
    if s.len() <= 8 {
        return "*".repeat(s.len());
    }
    let start = &s[..4];
    let end = &s[s.len() - 4..];
    format!("{}****{}", start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value() {
        let result = parse_key_value("webhook=https://example.com").unwrap();
        assert_eq!(result.0, "webhook");
        assert_eq!(result.1, "https://example.com");
    }

    #[test]
    fn test_mask_sensitive_value() {
        let value = serde_json::Value::String("sk-1234567890abcdef".to_string());
        let masked = mask_sensitive_value(&value);
        assert_eq!(masked, "sk-1****cdef");
    }
}
