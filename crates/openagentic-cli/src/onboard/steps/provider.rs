//! Provider Step - AI 提供商选择

use dialoguer::Select;
use crate::onboard::state::{WizardState, StepResult};

pub struct ProviderStep;

impl ProviderStep {
    pub fn new() -> Self {
        Self
    }

    pub fn run(state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n🤖 选择 AI 模型提供商\n");

        let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .items(&[
                "OpenAI",
                "Anthropic (Claude)",
                "DeepSeek",
                "通义千问 (Qwen)",
                "智谱 GLM",
                "Moonshot (Kimi)",
                "MiniMax",
                "豆包 (Doubao)",
                "Google (Gemini)",
                "自定义 (OpenAI兼容)",
            ])
            .with_prompt("请选择 AI 提供商")
            .default(0)
            .interact()
            .unwrap_or(0);

        let (provider_name, models): (&str, Vec<&str>) = match selection {
            0 => ("OpenAI", vec!["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-preview"]),
            1 => ("Anthropic (Claude)", vec!["claude-4-opus", "claude-4-sonnet", "claude-3.5-sonnet", "claude-3-opus"]),
            2 => ("DeepSeek", vec!["deepseek-chat", "deepseek-coder", "deepseek-reasoner"]),
            3 => ("通义千问 (Qwen)", vec!["qwen-max", "qwen-plus", "qwen-turbo", "qwen-vl-max"]),
            4 => ("智谱 GLM", vec!["glm-4", "glm-4-plus", "glm-3-turbo"]),
            5 => ("Moonshot (Kimi)", vec!["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"]),
            6 => ("MiniMax", vec!["abab6.5s-chat", "abab6.5g-chat"]),
            7 => ("豆包 (Doubao)", vec!["doubao-lite", "doubao-pro"]),
            8 => ("Google (Gemini)", vec!["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"]),
            _ => ("自定义 (OpenAI兼容)", vec!["gpt-4", "claude-3"]),
        };
        
        state.provider = provider_name.to_string();
        
        println!("\n📋 选择模型:");
        let model_selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .items(&models)
            .with_prompt("请选择模型")
            .default(0)
            .interact()
            .unwrap_or(0);
        
        state.model = models[model_selection].to_string();

        if selection < 9 {
            println!("\n🔑 输入 API Key:");
            print!("请输入 API Key: ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            let mut api_key = String::new();
            std::io::stdin().read_line(&mut api_key).ok();
            let api_key = api_key.trim().to_string();
            if api_key.is_empty() {
                return Ok(StepResult::failure("API Key 不能为空"));
            }
            state.api_key = Some(api_key);
        } else {
            println!("\n🔑 自定义 API 配置:");
            print!("API Base URL (如: https://api.openai.com/v1): ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            let mut api_base = String::new();
            std::io::stdin().read_line(&mut api_base).ok();
            state.api_base = Some(api_base.trim().to_string());
            
            print!("API Key: ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            let mut api_key = String::new();
            std::io::stdin().read_line(&mut api_key).ok();
            let api_key = api_key.trim().to_string();
            if api_key.is_empty() {
                return Ok(StepResult::failure("API Key 不能为空"));
            }
            state.api_key = Some(api_key);
        }

        println!("\n✅ AI 提供商配置完成");
        Ok(StepResult::success())
    }
}
