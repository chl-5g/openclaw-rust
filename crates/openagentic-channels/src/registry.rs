use std::sync::Arc;

use crate::config::ChannelConfigs;
use crate::factory::ChannelFactoryRegistry;
use openagentic_core::Result;

pub async fn register_default_channels(registry: &ChannelFactoryRegistry) {
    register_telegram(registry).await;
    register_discord(registry).await;
    register_slack(registry).await;
    register_teams(registry).await;
    register_feishu(registry).await;
    register_wecom(registry).await;
    register_dingtalk(registry).await;
    register_whatsapp(registry).await;
}

pub async fn register_channels_from_config(
    registry: &ChannelFactoryRegistry,
    config: &ChannelConfigs,
) {
    for (name, entry) in config.0.iter() {
        if !entry.enabled {
            tracing::debug!("Channel '{}' is disabled, skipping", name);
            continue;
        }
        
        match name.as_str() {
            "telegram" => register_telegram_with_config(registry, entry.config.clone()).await,
            "discord" => register_discord_with_config(registry, entry.config.clone()).await,
            "slack" => register_slack_with_config(registry, entry.config.clone()).await,
            "teams" => register_teams_with_config(registry, entry.config.clone()).await,
            "feishu" => register_feishu_with_config(registry, entry.config.clone()).await,
            "wecom" => register_wecom_with_config(registry, entry.config.clone()).await,
            "dingtalk" => register_dingtalk_with_config(registry, entry.config.clone()).await,
            "whatsapp" => register_whatsapp_with_config(registry, entry.config.clone()).await,
            _ => tracing::warn!("Unknown channel type: {}", name),
        }
    }
}

async fn register_telegram_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::telegram::{TelegramBot, TelegramConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let telegram_config = if let Some(obj) = config.as_object() {
                TelegramConfig {
                    bot_token: obj.get("bot_token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    enabled: obj.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                TelegramConfig {
                    bot_token: String::new(),
                    enabled: false,
                }
            };
            let bot = TelegramBot::new(telegram_config);
            Ok(Arc::new(tokio::sync::RwLock::new(bot)))
        };
    registry.register("telegram".to_string(), creator).await;
    tracing::info!("Registered Telegram channel from config");
}

async fn register_discord_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::discord::{DiscordChannel, DiscordConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let discord_config = if let Some(obj) = config.as_object() {
                DiscordConfig {
                    bot_token: obj.get("bot_token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    webhook_url: obj.get("webhook_url")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    enabled: obj.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    #[cfg(feature = "discord")]
                    use_gateway: obj.get("use_gateway")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                DiscordConfig {
                    bot_token: String::new(),
                    webhook_url: None,
                    enabled: false,
                    #[cfg(feature = "discord")]
                    use_gateway: false,
                }
            };
            let channel = DiscordChannel::new(discord_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("discord".to_string(), creator).await;
    tracing::info!("Registered Discord channel from config");
}

async fn register_slack_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::slack::{SlackChannel, SlackConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let slack_config = if let Some(obj) = config.as_object() {
                SlackConfig {
                    bot_token: obj.get("bot_token").and_then(|v| v.as_str()).map(String::from),
                    webhook_url: obj.get("webhook_url").and_then(|v| v.as_str()).map(String::from),
                    app_token: obj.get("app_token").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                SlackConfig {
                    bot_token: None,
                    webhook_url: None,
                    app_token: None,
                    enabled: false,
                }
            };
            let channel = SlackChannel::new(slack_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("slack".to_string(), creator).await;
    tracing::info!("Registered Slack channel from config");
}

async fn register_teams_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::teams::{TeamsChannel, TeamsConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let teams_config = if let Some(obj) = config.as_object() {
                TeamsConfig {
                    webhook_url: obj.get("webhook_url").and_then(|v| v.as_str()).map(String::from),
                    bot_id: obj.get("bot_id").and_then(|v| v.as_str()).map(String::from),
                    bot_password: obj.get("bot_password").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                TeamsConfig {
                    webhook_url: None,
                    bot_id: None,
                    bot_password: None,
                    enabled: false,
                }
            };
            let channel = TeamsChannel::new(teams_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("teams".to_string(), creator).await;
    tracing::info!("Registered Teams channel from config");
}

async fn register_feishu_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::feishu::{FeishuChannel, FeishuConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let feishu_config = if let Some(obj) = config.as_object() {
                FeishuConfig {
                    app_id: obj.get("app_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    app_secret: obj.get("app_secret").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                FeishuConfig {
                    app_id: String::new(),
                    app_secret: String::new(),
                    webhook: None,
                    enabled: false,
                }
            };
            let channel = FeishuChannel::new(feishu_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("feishu".to_string(), creator).await;
    tracing::info!("Registered Feishu channel from config");
}

async fn register_wecom_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::wecom::{WeComChannel, WeComConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let wecom_config = if let Some(obj) = config.as_object() {
                WeComConfig {
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                WeComConfig {
                    webhook: String::new(),
                    enabled: false,
                }
            };
            let channel = WeComChannel::new(wecom_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("wecom".to_string(), creator).await;
    tracing::info!("Registered WeCom channel from config");
}

async fn register_dingtalk_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::dingtalk::{DingTalkChannel, DingTalkConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = if let Some(obj) = config.as_object() {
                DingTalkConfig {
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    secret: obj.get("secret").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                DingTalkConfig {
                    webhook: String::new(),
                    secret: None,
                    enabled: false,
                }
            };
            let channel = DingTalkChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("dingtalk".to_string(), creator).await;
    tracing::info!("Registered DingTalk channel from config");
}

async fn register_whatsapp_with_config(
    registry: &ChannelFactoryRegistry,
    config: serde_json::Value,
) {
    use crate::whatsapp::{WhatsAppChannel, WhatsAppConfig};

    let creator =
        move |_cfg: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = if let Some(obj) = config.as_object() {
                WhatsAppConfig {
                    business_account_id: obj.get("business_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    phone_number_id: obj.get("phone_number_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    access_token: obj.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    verify_token: obj.get("verify_token").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                WhatsAppConfig {
                    business_account_id: String::new(),
                    phone_number_id: String::new(),
                    access_token: String::new(),
                    verify_token: None,
                    enabled: false,
                }
            };
            let channel = WhatsAppChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("whatsapp".to_string(), creator).await;
    tracing::info!("Registered WhatsApp channel from config");
}

async fn register_telegram(registry: &ChannelFactoryRegistry) {
    use crate::telegram::{TelegramBot, TelegramConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let telegram_config = if let Some(obj) = config.as_object() {
                TelegramConfig {
                    bot_token: obj.get("bot_token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    enabled: obj.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                TelegramConfig {
                    bot_token: String::new(),
                    enabled: false,
                }
            };
            let bot = TelegramBot::new(telegram_config);
            Ok(Arc::new(tokio::sync::RwLock::new(bot)))
        };
    registry.register("telegram".to_string(), creator).await;
}

async fn register_discord(registry: &ChannelFactoryRegistry) {
    use crate::discord::{DiscordChannel, DiscordConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let discord_config = if let Some(obj) = config.as_object() {
                DiscordConfig {
                    bot_token: obj.get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    webhook_url: obj.get("webhook_url")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    enabled: obj.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    #[cfg(feature = "discord")]
                    use_gateway: obj.get("use_gateway")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                DiscordConfig {
                    bot_token: String::new(),
                    webhook_url: None,
                    enabled: true,
                    #[cfg(feature = "discord")]
                    use_gateway: false,
                }
            };
            let channel = DiscordChannel::new(discord_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("discord".to_string(), creator).await;
}

async fn register_slack(registry: &ChannelFactoryRegistry) {
    use crate::slack::{SlackChannel, SlackConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let slack_config = if let Some(obj) = config.as_object() {
                SlackConfig {
                    bot_token: obj.get("bot_token").and_then(|v| v.as_str()).map(String::from),
                    webhook_url: obj.get("webhook_url").and_then(|v| v.as_str()).map(String::from),
                    app_token: obj.get("app_token").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                SlackConfig {
                    bot_token: None,
                    webhook_url: None,
                    app_token: None,
                    enabled: false,
                }
            };
            let channel = SlackChannel::new(slack_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("slack".to_string(), creator).await;
}

async fn register_teams(registry: &ChannelFactoryRegistry) {
    use crate::teams::{TeamsChannel, TeamsConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let teams_config = if let Some(obj) = config.as_object() {
                TeamsConfig {
                    webhook_url: obj.get("webhook_url").and_then(|v| v.as_str()).map(String::from),
                    bot_id: obj.get("bot_id").and_then(|v| v.as_str()).map(String::from),
                    bot_password: obj.get("bot_password").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                TeamsConfig {
                    webhook_url: None,
                    bot_id: None,
                    bot_password: None,
                    enabled: false,
                }
            };
            let channel = TeamsChannel::new(teams_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("teams".to_string(), creator).await;
}

async fn register_feishu(registry: &ChannelFactoryRegistry) {
    use crate::feishu::{FeishuChannel, FeishuConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let feishu_config = if let Some(obj) = config.as_object() {
                FeishuConfig {
                    app_id: obj.get("app_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    app_secret: obj.get("app_secret").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                FeishuConfig {
                    app_id: String::new(),
                    app_secret: String::new(),
                    webhook: None,
                    enabled: false,
                }
            };
            let channel = FeishuChannel::new(feishu_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("feishu".to_string(), creator).await;
}

async fn register_wecom(registry: &ChannelFactoryRegistry) {
    use crate::wecom::{WeComChannel, WeComConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let wecom_config = if let Some(obj) = config.as_object() {
                WeComConfig {
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                WeComConfig {
                    webhook: String::new(),
                    enabled: false,
                }
            };
            let channel = WeComChannel::new(wecom_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("wecom".to_string(), creator).await;
}

async fn register_dingtalk(registry: &ChannelFactoryRegistry) {
    use crate::dingtalk::{DingTalkChannel, DingTalkConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = if let Some(obj) = config.as_object() {
                DingTalkConfig {
                    webhook: obj.get("webhook").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    secret: obj.get("secret").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                DingTalkConfig {
                    webhook: String::new(),
                    secret: None,
                    enabled: false,
                }
            };
            let channel = DingTalkChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("dingtalk".to_string(), creator).await;
}

async fn register_whatsapp(registry: &ChannelFactoryRegistry) {
    use crate::whatsapp::{WhatsAppChannel, WhatsAppConfig};

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = if let Some(obj) = config.as_object() {
                WhatsAppConfig {
                    business_account_id: obj.get("business_account_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    phone_number_id: obj.get("phone_number_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    access_token: obj.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    verify_token: obj.get("verify_token").and_then(|v| v.as_str()).map(String::from),
                    enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            } else {
                WhatsAppConfig {
                    business_account_id: String::new(),
                    phone_number_id: String::new(),
                    access_token: String::new(),
                    verify_token: None,
                    enabled: false,
                }
            };
            let channel = WhatsAppChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("whatsapp".to_string(), creator).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_default_channels() {
        let registry = ChannelFactoryRegistry::new();

        register_default_channels(&registry).await;

        let types = registry.list_types().await;
        assert!(types.contains(&"telegram".to_string()));
        assert!(types.contains(&"discord".to_string()));
    }

    #[tokio::test]
    async fn test_register_specific_channel() {
        let registry = ChannelFactoryRegistry::new();

        register_telegram(&registry).await;

        assert!(registry.contains("telegram").await);
        assert!(!registry.contains("discord").await);
    }

    #[tokio::test]
    async fn test_create_channel_from_registry() {
        let registry = ChannelFactoryRegistry::new();

        register_telegram(&registry).await;

        let config = serde_json::json!({
            "bot_token": "test_token",
            "enabled": true
        });

        let channel = registry.create("telegram", config).await;
        assert!(channel.is_ok());
    }
}
