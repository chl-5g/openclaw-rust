//! 语音模块类型定义

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::provider::CustomProviderConfig;

/// Vault trait for credential storage
#[async_trait::async_trait]
pub trait CredentialVault: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, String>;
    async fn set(&self, key: &str, value: &str) -> Result<(), String>;
    async fn delete(&self, key: &str) -> Result<(), String>;
}

/// Null vault implementation (no-op, for backward compatibility)
pub struct NullVault;

#[async_trait::async_trait]
impl CredentialVault for NullVault {
    async fn get(&self, _key: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    async fn set(&self, _key: &str, _value: &str) -> Result<(), String> {
        Ok(())
    }
    async fn delete(&self, _key: &str) -> Result<(), String> {
        Ok(())
    }
}

/// STT 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SttProvider {
    /// OpenAI Whisper API
    #[default]
    OpenAI,
    /// 本地 Whisper 模型
    LocalWhisper,
    /// Azure Speech
    Azure,
    /// Google Cloud Speech
    Google,
    /// 自定义提供商 (用户配置)
    Custom(String),
}

/// TTS 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TtsProvider {
    /// OpenAI TTS API
    #[default]
    OpenAI,
    /// Edge TTS (免费)
    Edge,
    /// Azure Speech
    Azure,
    /// Google Cloud TTS
    Google,
    /// ElevenLabs TTS
    ElevenLabs,
    /// Piper (本地, 轻量)
    Piper,
    /// Coqui TTS (本地, 高质量)
    Coqui,
    /// CosyVoice (本地 Docker)
    CosyVoice,
    /// macOS System TTS
    MacOSSystem,
    /// Linux System TTS (eSpeak/Festival)
    LinuxSystem,
    /// Windows System TTS (SAPI)
    WindowsSystem,
    /// 自定义提供商 (用户配置)
    Custom(String),
}

/// 语音识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// 识别的文本
    pub text: String,
    /// 语言（自动检测）
    pub language: Option<String>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// 持续时间（秒）
    pub duration: Option<f64>,
}

/// 语音合成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisOptions {
    /// 语音名称
    pub voice: Option<String>,
    /// 语速 (0.25 - 4.0)
    pub speed: Option<f32>,
    /// 音调 (仅部分提供商支持)
    pub pitch: Option<f32>,
    /// 输出格式
    pub format: Option<AudioFormat>,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            voice: None,
            speed: Some(1.0),
            pitch: None,
            format: Some(AudioFormat::Mp3),
        }
    }
}

/// 音频格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    #[default]
    Mp3,
    Wav,
    Ogg,
    Flac,
    Pcm,
}

impl AudioFormat {
    pub fn as_extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Flac => "flac",
            AudioFormat::Pcm => "pcm",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Pcm => "audio/pcm",
        }
    }
}

/// OpenAI TTS 可用语音
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum OpenAIVoice {
    #[default]
    Alloy,
    Echo,
    Fable,
    Onyx,
    Nova,
    Shimmer,
}

impl OpenAIVoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIVoice::Alloy => "alloy",
            OpenAIVoice::Echo => "echo",
            OpenAIVoice::Fable => "fable",
            OpenAIVoice::Onyx => "onyx",
            OpenAIVoice::Nova => "nova",
            OpenAIVoice::Shimmer => "shimmer",
        }
    }
}

/// OpenAI Whisper 可用模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum WhisperModel {
    #[default]
    Whisper1,
}

impl WhisperModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            WhisperModel::Whisper1 => "whisper-1",
        }
    }
}

/// OpenAI TTS 可用模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TtsModel {
    #[default]
    Tts1,
    Tts1Hd,
}

impl TtsModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsModel::Tts1 => "tts-1",
            TtsModel::Tts1Hd => "tts-1-hd",
        }
    }
}

/// Talk Mode 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TalkModeState {
    /// 空闲
    Idle,
    /// 监听中
    Listening,
    /// 处理中
    Processing,
    /// 播放回复中
    Speaking,
}

/// 语音配置
#[derive(Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// STT 提供商
    pub stt_provider: SttProvider,
    /// TTS 提供商
    pub tts_provider: TtsProvider,
    /// STT 配置
    #[serde(flatten)]
    pub stt_config: SttConfig,
    /// TTS 配置
    #[serde(flatten)]
    pub tts_config: TtsConfig,
    /// 是否启用
    pub enabled: bool,
    /// 自定义提供商配置
    #[serde(default)]
    pub custom_providers: Option<CustomProviderConfig>,
    /// Credential Vault for secure API key storage
    #[serde(skip)]
    pub vault: Option<Arc<dyn CredentialVault>>,
}

impl std::fmt::Debug for VoiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceConfig")
            .field("stt_provider", &self.stt_provider)
            .field("tts_provider", &self.tts_provider)
            .field("stt_config", &self.stt_config)
            .field("tts_config", &self.tts_config)
            .field("enabled", &self.enabled)
            .field("custom_providers", &self.custom_providers)
            .field("vault", &if self.vault.is_some() { "Some(...)" } else { "None" })
            .finish()
    }
}

impl VoiceConfig {
    pub fn with_vault(mut self, vault: Arc<dyn CredentialVault>) -> Self {
        let vault_clone = vault.clone();
        self.stt_config.vault = Some(vault_clone);
        let vault_clone = vault.clone();
        self.tts_config.vault = Some(vault_clone);
        self.vault = Some(vault);
        self
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_provider: SttProvider::OpenAI,
            tts_provider: TtsProvider::OpenAI,
            stt_config: SttConfig::default(),
            tts_config: TtsConfig::default(),
            enabled: false,
            custom_providers: None,
            vault: None,
        }
    }
}

/// STT 配置
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SttConfig {
    /// OpenAI API Key
    pub openai_api_key: Option<String>,
    /// OpenAI Base URL
    pub openai_base_url: Option<String>,
    /// Whisper 模型
    #[serde(default)]
    pub whisper_model: WhisperModel,
    /// 语言提示
    pub language: Option<String>,
    /// 本地模型路径
    pub local_model_path: Option<String>,
    /// Azure Speech API Key
    pub azure_api_key: Option<String>,
    /// Azure Speech 区域
    pub azure_region: Option<String>,
    /// Google Cloud API Key
    pub google_api_key: Option<String>,
    /// Credential Vault for secure API key storage
    #[serde(skip)]
    pub vault: Option<Arc<dyn CredentialVault>>,
}

impl std::fmt::Debug for SttConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SttConfig")
            .field("openai_api_key", &self.openai_api_key.as_deref().map(|_| "***"))
            .field("openai_base_url", &self.openai_base_url)
            .field("whisper_model", &self.whisper_model)
            .field("language", &self.language)
            .field("local_model_path", &self.local_model_path)
            .field("azure_api_key", &self.azure_api_key.as_deref().map(|_| "***"))
            .field("azure_region", &self.azure_region)
            .field("google_api_key", &self.google_api_key.as_deref().map(|_| "***"))
            .field("vault", &if self.vault.is_some() { "Some(...)" } else { "None" })
            .finish()
    }
}

/// TTS 配置
#[derive(Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// OpenAI API Key
    pub openai_api_key: Option<String>,
    /// OpenAI Base URL
    pub openai_base_url: Option<String>,
    /// TTS 模型
    #[serde(default)]
    pub tts_model: TtsModel,
    /// 默认语音
    #[serde(default)]
    pub default_voice: OpenAIVoice,
    /// 默认语速
    #[serde(default = "default_speed")]
    pub default_speed: f32,
    /// 默认格式
    #[serde(default)]
    pub default_format: AudioFormat,
    /// ElevenLabs API Key
    pub elevenlabs_api_key: Option<String>,
    /// ElevenLabs Model ID
    #[serde(default = "default_elevenlabs_model")]
    pub elevenlabs_model: String,
    /// Azure Speech API Key
    pub azure_api_key: Option<String>,
    /// Azure Speech 区域
    pub azure_region: Option<String>,
    /// Google Cloud API Key
    pub google_api_key: Option<String>,
    /// Credential Vault for secure API key storage
    #[serde(skip)]
    pub vault: Option<Arc<dyn CredentialVault>>,
}

impl std::fmt::Debug for TtsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TtsConfig")
            .field("openai_api_key", &self.openai_api_key.as_deref().map(|_| "***"))
            .field("openai_base_url", &self.openai_base_url)
            .field("tts_model", &self.tts_model)
            .field("default_voice", &self.default_voice)
            .field("default_speed", &self.default_speed)
            .field("default_format", &self.default_format)
            .field("elevenlabs_api_key", &self.elevenlabs_api_key.as_deref().map(|_| "***"))
            .field("elevenlabs_model", &self.elevenlabs_model)
            .field("azure_api_key", &self.azure_api_key.as_deref().map(|_| "***"))
            .field("azure_region", &self.azure_region)
            .field("google_api_key", &self.google_api_key.as_deref().map(|_| "***"))
            .field("vault", &if self.vault.is_some() { "Some(...)" } else { "None" })
            .finish()
    }
}

fn default_speed() -> f32 {
    1.0
}

fn default_elevenlabs_model() -> String {
    "eleven_multilingual_v2".to_string()
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            openai_base_url: None,
            tts_model: TtsModel::default(),
            default_voice: OpenAIVoice::default(),
            default_speed: 1.0,
            default_format: AudioFormat::Mp3,
            elevenlabs_api_key: None,
            elevenlabs_model: "eleven_multilingual_v2".to_string(),
            azure_api_key: None,
            azure_region: None,
            google_api_key: None,
            vault: None,
        }
    }
}

impl TtsConfig {
    pub fn with_vault(mut self, vault: Arc<dyn CredentialVault>) -> Self {
        self.vault = Some(vault);
        self
    }

    pub async fn get_openai_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("tts/openai_api_key").await? {
                return Ok(key);
            }
        }
        self.openai_api_key.clone()
            .ok_or_else(|| "OpenAI API key not configured".to_string())
    }

    pub async fn get_elevenlabs_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("tts/elevenlabs_api_key").await? {
                return Ok(key);
            }
        }
        self.elevenlabs_api_key.clone()
            .ok_or_else(|| "ElevenLabs API key not configured".to_string())
    }

    pub async fn get_azure_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("tts/azure_api_key").await? {
                return Ok(key);
            }
        }
        self.azure_api_key.clone()
            .ok_or_else(|| "Azure API key not configured".to_string())
    }

    pub async fn get_google_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("tts/google_api_key").await? {
                return Ok(key);
            }
        }
        self.google_api_key.clone()
            .ok_or_else(|| "Google API key not configured".to_string())
    }
}

impl SttConfig {
    pub fn with_vault(mut self, vault: Arc<dyn CredentialVault>) -> Self {
        self.vault = Some(vault);
        self
    }

    pub async fn get_openai_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("stt/openai_api_key").await? {
                return Ok(key);
            }
        }
        self.openai_api_key.clone()
            .ok_or_else(|| "OpenAI API key not configured".to_string())
    }

    pub async fn get_azure_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("stt/azure_api_key").await? {
                return Ok(key);
            }
        }
        self.azure_api_key.clone()
            .ok_or_else(|| "Azure API key not configured".to_string())
    }

    pub async fn get_google_api_key(&self) -> Result<String, String> {
        if let Some(ref vault) = self.vault {
            if let Some(key) = vault.get("stt/google_api_key").await? {
                return Ok(key);
            }
        }
        self.google_api_key.clone()
            .ok_or_else(|| "Google API key not configured".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug)]
    struct TestVault {
        store: Arc<RwLock<std::collections::HashMap<String, String>>>,
    }

    #[async_trait::async_trait]
    impl CredentialVault for TestVault {
        async fn get(&self, key: &str) -> Result<Option<String>, String> {
            let store = self.store.read().await;
            Ok(store.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &str) -> Result<(), String> {
            let mut store = self.store.write().await;
            store.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<(), String> {
            let mut store = self.store.write().await;
            store.remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_tts_config_vault_fallback_to_direct() {
        let config = TtsConfig {
            elevenlabs_api_key: Some("test-key-from-config".to_string()),
            vault: None,
            ..Default::default()
        };

        let key = config.get_elevenlabs_api_key().await.unwrap();
        assert_eq!(key, "test-key-from-config");
    }

    #[tokio::test]
    async fn test_tts_config_vault_takes_precedence() {
        let vault = Arc::new(TestVault {
            store: Arc::new(RwLock::new(std::collections::HashMap::new())),
        });

        vault.set("tts/elevenlabs_api_key", "key-from-vault").await.unwrap();

        let config = TtsConfig {
            elevenlabs_api_key: Some("test-key-from-config".to_string()),
            vault: Some(vault),
            ..Default::default()
        };

        let key = config.get_elevenlabs_api_key().await.unwrap();
        assert_eq!(key, "key-from-vault");
    }

    #[tokio::test]
    async fn test_tts_config_vault_not_found_falls_back_to_direct() {
        let vault = Arc::new(TestVault {
            store: Arc::new(RwLock::new(std::collections::HashMap::new())),
        });

        let config = TtsConfig {
            elevenlabs_api_key: Some("test-key-from-config".to_string()),
            vault: Some(vault),
            ..Default::default()
        };

        let key = config.get_elevenlabs_api_key().await.unwrap();
        assert_eq!(key, "test-key-from-config");
    }

    #[tokio::test]
    async fn test_tts_config_no_key_available() {
        let config = TtsConfig::default();

        let result = config.get_elevenlabs_api_key().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stt_config_vault_fallback_to_direct() {
        let config = SttConfig {
            openai_api_key: Some("test-key-from-config".to_string()),
            vault: None,
            ..Default::default()
        };

        let key = config.get_openai_api_key().await.unwrap();
        assert_eq!(key, "test-key-from-config");
    }

    #[tokio::test]
    async fn test_voice_config_with_vault() {
        let vault = Arc::new(TestVault {
            store: Arc::new(RwLock::new(std::collections::HashMap::new())),
        });

        vault.set("tts/elevenlabs_api_key", "voice-vault-key").await.unwrap();

        let config = VoiceConfig::default().with_vault(vault);

        let key = config.tts_config.get_elevenlabs_api_key().await.unwrap();
        assert_eq!(key, "voice-vault-key");
    }
}
