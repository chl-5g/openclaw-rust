//! Providers 子命令 - 提供商管理

use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ProvidersSubCmd {
    /// 列出所有可用提供商
    List,
    /// 添加新提供商 (2步: 输入名称，然后输入 API Key)
    Add {
        /// 提供商名称 (如 openai, anthropic, deepseek)
        name: Option<String>,
    },
    /// 删除提供商
    Remove {
        /// 提供商名称
        name: String,
    },
    /// 显示提供商配置
    Show {
        /// 提供商名称
        name: Option<String>,
    },
    /// 测试提供商连接
    Test {
        /// 提供商名称
        name: Option<String>,
    },
}

impl ProvidersSubCmd {
    pub async fn run(&self) -> Result<()> {
        match self {
            ProvidersSubCmd::List => self.list().await,
            ProvidersSubCmd::Add { name } => self.add(name.clone()).await,
            ProvidersSubCmd::Remove { name } => self.remove(name).await,
            ProvidersSubCmd::Show { name } => self.show(name.clone()).await,
            ProvidersSubCmd::Test { name } => self.test(name.clone()).await,
        }
    }

    async fn list(&self) -> Result<()> {
        println!();
        println!("\x1b[36m\x1b[1m🤖 Available AI Providers\x1b[0m");
        println!();
        
        let providers = vec![
            ("openai", "OpenAI", "GPT-4o, GPT-4, GPT-3.5", "api.openai.com"),
            ("anthropic", "Anthropic", "Claude-3.5, Claude-3", "api.anthropic.com"),
            ("google", "Google AI", "Gemini Pro", "generativelanguage.googleapis.com"),
            ("azure", "Azure OpenAI", "GPT-4, GPT-3.5", "*.openai.azure.com"),
            ("deepseek", "DeepSeek", "DeepSeek Coder, Chat", "api.deepseek.com"),
            ("ollama", "Ollama", "Local models (Llama, Mistral)", "localhost:11434"),
            ("moonshot", "Moonshot AI", "Moonshot v1", "api.moonshot.cn"),
            ("zhipu", "智谱 AI", "GLM-4, GLM-3", "open.bigmodel.cn"),
            ("minimax", "MiniMax", "Abab6, Text-01", "api.minimax.chat"),
        ];

        for (name, display, models, endpoint) in providers {
            println!("  \x1b[33m{}\x1b[0m", name);
            println!("    \x1b[90m{} | Models: {} | Endpoint: {}\x1b[0m", display, models, endpoint);
        }

        println!();
        
        let config = openagentic_core::UnifiedConfig::load(&openagentic_core::UnifiedConfig::default_path())
            .unwrap_or_default();
        
        if !config.providers.entries.is_empty() {
            println!("\x1b[36m\x1b[1m📦 Configured Providers\x1b[0m");
            println!();
            for (name, entry) in &config.providers.entries {
                let has_key = match entry {
                    openagentic_core::config_loader::ProviderEntry::WithKey { .. } => true,
                    openagentic_core::config_loader::ProviderEntry::NoKey { .. } => false,
                };
                let status = if has_key {
                    "\x1b[32m✓ Configured\x1b[0m"
                } else {
                    "\x1b[33m⚠ No API Key\x1b[0m"
                };
                println!("  \x1b[33m{}\x1b[0m  {}", name, status);
            }
            println!();
        }

        println!("Usage:");
        println!("  \x1b[36mopen-agentic providers add openai\x1b[0m   - Add a new provider");
        println!("  \x1b[36mopen-agentic providers show openai\x1b[0m - Show provider details");
        println!();

        Ok(())
    }

    async fn add(&self, name: Option<String>) -> Result<()> {
        let provider_name = if let Some(n) = name {
            n
        } else {
            println!();
            println!("\x1b[36m\x1b[1m➕ Add New Provider\x1b[0m");
            println!();
            println!("Available providers:");
            println!("  openai      - OpenAI (GPT-4o, GPT-4)");
            println!("  anthropic  - Anthropic (Claude-3.5)");
            println!("  google     - Google AI (Gemini)");
            println!("  deepseek   - DeepSeek");
            println!("  ollama     - Ollama (local)");
            println!("  moonshot   - Moonshot AI");
            println!("  zhipu      - 智谱 AI");
            println!("  minimax    - MiniMax");
            println!();
            print!("Enter provider name: ");
            
            use std::io::{self, Write};
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        };

        if provider_name.is_empty() {
            println!("\x1b[31mError: Provider name cannot be empty\x1b[0m");
            return Ok(());
        }

        println!();
        println!("\x1b[33mAdding provider: {}\x1b[0m", provider_name);
        println!();

        print!("Enter API Key: ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut api_key = String::new();
        io::stdin().read_line(&mut api_key)?;
        api_key = api_key.trim().to_string();

        if api_key.is_empty() {
            println!("\x1b[31mError: API Key cannot be empty\x1b[0m");
            return Ok(());
        }

        let config_path = openagentic_core::UnifiedConfig::default_path();
        let mut config = openagentic_core::UnifiedConfig::load(&config_path).unwrap_or_default();

        config.providers.entries.insert(
            provider_name.clone(),
            openagentic_core::config_loader::ProviderEntry::WithKey { 
                api_key, 
                api_base: None 
            },
        );

        config.save(&config_path)?;

        println!();
        println!("\x1b[32m✅ Provider '{}' added successfully!\x1b[0m", provider_name);
        println!();
        println!("Next steps:");
        println!("  \x1b[36mopen-agentic providers test {}\x1b[0m  - Test the connection", provider_name);
        println!();

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let config_path = openagentic_core::UnifiedConfig::default_path();
        let mut config = openagentic_core::UnifiedConfig::load(&config_path).unwrap_or_default();

        if config.providers.entries.remove(name).is_some() {
            config.save(&config_path)?;
            println!("\x1b[32m✅ Provider '{}' removed\x1b[0m", name);
        } else {
            println!("\x1b[31mProvider '{}' not found\x1b[0m", name);
        }

        Ok(())
    }

    async fn show(&self, name: Option<String>) -> Result<()> {
        let config = openagentic_core::UnifiedConfig::load(&openagentic_core::UnifiedConfig::default_path())
            .unwrap_or_default();

        if let Some(n) = name {
            println!();
            if let Some(entry) = config.providers.entries.get(&n) {
                println!("\x1b[36m\x1b[1mProvider: {}\x1b[0m", n);
                match entry {
                    openagentic_core::config_loader::ProviderEntry::WithKey { api_key, api_base } => {
                        println!("  Status: \x1b[32m✓ Configured\x1b[0m");
                        println!("  API Key: \x1b[33m{}\x1b[0m", mask_key(api_key));
                        if let Some(base) = api_base {
                            println!("  Endpoint: {}", base);
                        }
                    }
                    openagentic_core::config_loader::ProviderEntry::NoKey { api_base } => {
                        println!("  Status: \x1b[33m⚠ No API Key\x1b[0m");
                        if let Some(base) = api_base {
                            println!("  Endpoint: {}", base);
                        }
                    }
                }
            } else {
                println!("\x1b[31mProvider '{}' not found\x1b[0m", n);
            }
            println!();
        } else {
            self.list().await?;
        }

        Ok(())
    }

    async fn test(&self, name: Option<String>) -> Result<()> {
        let provider_name = if let Some(n) = name {
            n
        } else {
            self.list().await?;
            print!("\nEnter provider name to test: ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        };

        if provider_name.is_empty() {
            return Ok(());
        }

        println!();
        println!("\x1b[33mTesting provider: {}\x1b[0m", provider_name);
        println!();

        let config = openagentic_core::UnifiedConfig::load(&openagentic_core::UnifiedConfig::default_path())
            .unwrap_or_default();

        if let Some(entry) = config.providers.entries.get(&provider_name) {
            let api_key = match entry {
                openagentic_core::config_loader::ProviderEntry::WithKey { api_key, .. } => api_key,
                openagentic_core::config_loader::ProviderEntry::NoKey { .. } => {
                    println!("\x1b[31mError: No API Key configured for '{}'\x1b[0m", provider_name);
                    return Ok(());
                }
            };

            println!("\x1b[90mAPI Key: {}\x1b[0m", mask_key(api_key));
            println!("\x1b[90mTesting connection...\x1b[0m");
            
            println!();
            println!("\x1b[33mNote: Connection testing requires actual API call.\x1b[0m");
            println!("\x1b[90mThis is a placeholder for actual connectivity test.\x1b[0m");
            println!();
            
            println!("\x1b[32m✓ Configuration appears valid\x1b[0m");
        } else {
            println!("\x1b[31mProvider '{}' not found\x1b[0m", provider_name);
        }

        Ok(())
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    }
}
