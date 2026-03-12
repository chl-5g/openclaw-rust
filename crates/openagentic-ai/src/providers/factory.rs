//! AI 提供商工厂 — 所有厂商统一走 LiteLLM

use std::fmt;
use std::sync::Arc;

use super::{AIProvider, LiteLLMProvider, ProviderConfig};

/// 提供商类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Qwen,
    Doubao,
    Glm,
    Minimax,
    Kimi,
    OpenRouter,
    Ollama,
    Custom,
}

impl ProviderType {
    /// 从字符串解析提供商类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "deepseek" => Some(Self::DeepSeek),
            "qwen" | "alibaba" => Some(Self::Qwen),
            "doubao" | "bytedance" => Some(Self::Doubao),
            "glm" | "zhipu" => Some(Self::Glm),
            "minimax" => Some(Self::Minimax),
            "kimi" | "moonshot" => Some(Self::Kimi),
            "openrouter" => Some(Self::OpenRouter),
            "ollama" | "local" => Some(Self::Ollama),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// 获取默认模型
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-4o",
            Self::Anthropic => "claude-sonnet-4-20250514",
            Self::Gemini => "gemini-2.0-flash",
            Self::DeepSeek => "deepseek-chat",
            Self::Qwen => "qwen-plus",
            Self::Doubao => "doubao-pro-32k",
            Self::Glm => "glm-4-plus",
            Self::Minimax => "abab6.5s-chat",
            Self::Kimi => "moonshot-v1-8k",
            Self::OpenRouter => "openai/gpt-4o",
            Self::Ollama => "llama3.1",
            Self::Custom => "gpt-4o",
        }
    }

    /// litellm-rs 模型前缀
    fn model_prefix(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai/",
            Self::Anthropic => "anthropic/",
            Self::Gemini => "gemini/",
            Self::DeepSeek => "deepseek/",
            Self::Qwen => "openai/",   // Qwen 走 OpenAI 兼容, base_url 不同
            Self::Doubao => "openai/",  // 豆包走 OpenAI 兼容
            Self::Glm => "openai/",     // 智谱走 OpenAI 兼容
            Self::Minimax => "openai/", // MiniMax 走 OpenAI 兼容
            Self::Kimi => "openai/",    // Kimi 走 OpenAI 兼容
            Self::OpenRouter => "openrouter/",
            Self::Ollama => "ollama/",
            Self::Custom => "",         // 自定义不加前缀
        }
    }

    /// 默认 base_url（国内厂商需要）
    fn default_base_url(&self) -> Option<&'static str> {
        match self {
            Self::Qwen => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            Self::Doubao => Some("https://ark.cn-beijing.volces.com/api/v3"),
            Self::Glm => Some("https://open.bigmodel.cn/api/paas/v4"),
            Self::Minimax => Some("https://api.minimax.chat/v1"),
            Self::Kimi => Some("https://api.moonshot.cn/v1"),
            Self::Ollama => Some("http://localhost:11434"),
            _ => None,
        }
    }
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Qwen => "qwen",
            Self::Doubao => "doubao",
            Self::Glm => "glm",
            Self::Minimax => "minimax",
            Self::Kimi => "kimi",
            Self::OpenRouter => "openrouter",
            Self::Ollama => "ollama",
            Self::Custom => "custom",
        };
        write!(f, "{}", name)
    }
}

/// 提供商工厂 — 统一通过 LiteLLMProvider 创建
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据配置创建提供商实例
    pub fn create(
        provider_type: ProviderType,
        mut config: ProviderConfig,
    ) -> Result<Arc<dyn AIProvider>, String> {
        // 如果没设 base_url，用厂商默认值
        if config.base_url.is_none() {
            if let Some(url) = provider_type.default_base_url() {
                config.base_url = Some(url.to_string());
            }
        }

        // 如果没设默认模型，用厂商默认值
        if config.default_model.is_empty() {
            config.default_model = provider_type.default_model().to_string();
        }

        Ok(Arc::new(LiteLLMProvider::new(
            config,
            provider_type.model_prefix(),
        )))
    }

    /// 从提供商名称字符串创建提供商
    pub fn create_from_name(
        name: &str,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Result<Arc<dyn AIProvider>, String> {
        let provider_type =
            ProviderType::from_str(name).ok_or_else(|| format!("Unknown provider: {}", name))?;

        let mut config = ProviderConfig::new(name, api_key.unwrap_or_default());

        if let Some(url) = base_url {
            config = config.with_base_url(url);
        }

        config = config.with_default_model(provider_type.default_model());

        Self::create(provider_type, config)
    }

    /// 获取所有支持的提供商列表
    pub fn supported_providers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("openai", "OpenAI (GPT-4o, o1, o3)"),
            ("anthropic", "Anthropic (Claude 4)"),
            ("gemini", "Google Gemini"),
            ("deepseek", "DeepSeek"),
            ("qwen", "Alibaba Qwen (通义千问)"),
            ("doubao", "ByteDance Doubao (豆包)"),
            ("glm", "Zhipu GLM (智谱)"),
            ("minimax", "MiniMax"),
            ("kimi", "Moonshot Kimi (月之暗面)"),
            ("openrouter", "OpenRouter (100+ models)"),
            ("ollama", "Ollama (Local models)"),
            ("custom", "Custom (user-defined)"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(ProviderType::from_str("openai"), Some(ProviderType::OpenAI));
        assert_eq!(ProviderType::from_str("OpenAI"), Some(ProviderType::OpenAI));
        assert_eq!(
            ProviderType::from_str("anthropic"),
            Some(ProviderType::Anthropic)
        );
        assert_eq!(
            ProviderType::from_str("claude"),
            Some(ProviderType::Anthropic)
        );
        assert_eq!(ProviderType::from_str("ollama"), Some(ProviderType::Ollama));
        assert_eq!(ProviderType::from_str("local"), Some(ProviderType::Ollama));
        assert_eq!(ProviderType::from_str("unknown_provider"), None);
    }

    #[test]
    fn test_provider_type_default_model() {
        assert_eq!(ProviderType::OpenAI.default_model(), "gpt-4o");
        assert_eq!(
            ProviderType::Anthropic.default_model(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(ProviderType::Ollama.default_model(), "llama3.1");
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(ProviderType::OpenAI.to_string(), "openai");
        assert_eq!(ProviderType::Anthropic.to_string(), "anthropic");
        assert_eq!(ProviderType::Ollama.to_string(), "ollama");
    }

    #[test]
    fn test_supported_providers() {
        let providers = ProviderFactory::supported_providers();
        assert!(providers.iter().any(|(name, _)| *name == "openai"));
        assert!(providers.iter().any(|(name, _)| *name == "anthropic"));
        assert!(providers.iter().any(|(name, _)| *name == "ollama"));
        assert_eq!(providers.len(), 12);
    }

    #[test]
    fn test_create_provider() {
        let config = ProviderConfig::new("ollama", "")
            .with_base_url("http://localhost:11434")
            .with_default_model("qwen3:14b");
        let provider = ProviderFactory::create(ProviderType::Ollama, config);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "ollama");
    }

    #[test]
    fn test_create_from_name() {
        let provider =
            ProviderFactory::create_from_name("ollama", None, Some("http://localhost:11434".into()));
        assert!(provider.is_ok());
    }
}
