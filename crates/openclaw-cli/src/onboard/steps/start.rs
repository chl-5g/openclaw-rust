//! Start Step - 保存配置并启动服务

use dialoguer::Confirm;
use crate::onboard::state::{WizardState, StepResult};

pub struct StartStep;

impl StartStep {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n📝 配置摘要\n");
        
        println!("  用户: {}", state.user_name);
        println!("  AI 提供商: {}", state.provider);
        println!("  模型: {}", state.model);
        println!("  沙箱: {} ({})", 
            state.sandbox_type,
            if state.sandbox_enabled { "启用" } else { "禁用" });
        println!("  消息策略: {}", state.dm_policy);
        
        if !state.channels.is_empty() {
            println!("  通道: {} 个", state.channels.len());
        }
        
        if state.voice_enabled {
            println!("  语音: 启用");
        }
        
        if state.browser_enabled {
            println!("  浏览器: 启用");
        }

        println!("\n💾 保存配置...\n");
        
        if let Err(e) = save_config(state) {
            println!("   ❌ 保存失败: {}", e);
            return Ok(StepResult::failure(format!("保存配置失败: {}", e)));
        }
        
        println!("   ✅ 配置已保存到 ~/.openclaw-rust/openclaw.json");

        println!("\n🎉 初始化完成！\n");
        println!("  下一步:\n");
        println!("    • 运行 `openclaw-rust gateway` 启动服务");
        println!("    • 运行 `openclaw-rust doctor` 检查状态");
        println!("    • 查看文档: README.md\n");

        Ok(StepResult::success())
    }
}

fn save_config(state: &WizardState) -> anyhow::Result<()> {
    use std::fs;
    use std::path::PathBuf;
    
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw-rust");
    
    fs::create_dir_all(&config_dir)?;
    
    let config_path = config_dir.join("openclaw.json");
    
    let provider_key = match state.provider.as_str() {
        "OpenAI" => "openai",
        "Anthropic (Claude)" => "anthropic",
        "DeepSeek" => "deepseek",
        "通义千问 (Qwen)" => "qwen",
        "智谱 GLM" => "zhipu",
        "Moonshot (Kimi)" => "moonshot",
        "MiniMax" => "minimax",
        "豆包 (Doubao)" => "doubao",
        "Google (Gemini)" => "google",
        _ => "custom",
    };
    
    let mut provider_config = serde_json::Map::new();
    if let Some(ref key) = state.api_key {
        provider_config.insert("api_key".to_string(), serde_json::Value::String(key.clone()));
    }
    if let Some(ref base) = state.api_base {
        provider_config.insert("api_base".to_string(), serde_json::Value::String(base.clone()));
    }
    
    let mut providers = serde_json::Map::new();
    providers.insert(provider_key.to_string(), serde_json::Value::Object(provider_config));
    
    let config = serde_json::json!({
        "user_name": state.user_name,
        "default_provider": provider_key,
        "default_model": state.model,
        "providers": providers,
        "sandbox": {
            "enabled": state.sandbox_enabled,
            "default_type": state.sandbox_type,
            "timeout_secs": 30,
            "memory_limit_mb": 64
        },
        "security": {
            "dm_policy": state.dm_policy
        }
    });
    
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, content)?;
    
    Ok(())
}
