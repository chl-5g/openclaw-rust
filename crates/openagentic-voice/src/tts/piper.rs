//! Piper TTS - Local lightweight TTS engine
//! 
//! Piper is a fast, local TTS system. Visit https://github.com/rhasspy/piper for installation.

use crate::types::{TtsConfig, TtsProvider};
use async_trait::async_trait;
use openagentic_core::Result;

pub struct PiperTts {
    config: TtsConfig,
}

impl PiperTts {
    pub fn new(config: TtsConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl crate::tts::TextToSpeech for PiperTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::Piper
    }

    async fn synthesize(&self, text: &str, _options: Option<crate::types::SynthesisOptions>) -> Result<Vec<u8>> {
        Err(openagentic_core::OpenAgenticError::NotImplemented(
            "Piper TTS requires the piper binary to be installed. Visit https://github.com/rhasspy/piper for installation.".to_string()
        ))
    }

    async fn is_available(&self) -> bool {
        std::process::Command::new("piper")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn available_voices(&self) -> Vec<String> {
        vec!["en_US-lessac-medium".to_string()]
    }
}
