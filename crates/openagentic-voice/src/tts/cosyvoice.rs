//! CosyVoice - Local Docker-based TTS
//! 
//! CosyVoice is a local TTS system that runs in Docker. 
//! Visit https://github.com/Five大街/CosyVoice for details.

use crate::types::{TtsConfig, TtsProvider};
use async_trait::async_trait;
use openagentic_core::Result;

pub struct CosyVoiceTts {
    config: TtsConfig,
}

impl CosyVoiceTts {
    pub fn new(config: TtsConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl crate::tts::TextToSpeech for CosyVoiceTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::CosyVoice
    }

    async fn synthesize(&self, text: &str, _options: Option<crate::types::SynthesisOptions>) -> Result<Vec<u8>> {
        Err(openagentic_core::OpenAgenticError::NotImplemented(
            "CosyVoice requires Docker to be running. Start CosyVoice container and configure the endpoint.".to_string()
        ))
    }

    async fn is_available(&self) -> bool {
        std::process::Command::new("docker")
            .args(["ps", "--filter", "name=cosyvoice", "--format", "{{.Names}}"])
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("cosyvoice"))
            .unwrap_or(false)
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "中文女".to_string(),
            "中文男".to_string(),
            "日语女".to_string(),
            "日语男".to_string(),
            "韩语女".to_string(),
        ]
    }
}
