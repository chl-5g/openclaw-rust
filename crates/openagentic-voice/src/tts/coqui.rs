//! Coqui TTS - High-quality local TTS engine
//! 
//! Coqui TTS is an open-source neural TTS system. Visit https://github.com/coqui-ai/TTS for details.

use crate::types::{TtsConfig, TtsProvider};
use async_trait::async_trait;
use openagentic_core::Result;

pub struct CoquiTts {
    config: TtsConfig,
}

impl CoquiTts {
    pub fn new(config: TtsConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl crate::tts::TextToSpeech for CoquiTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::Coqui
    }

    async fn synthesize(&self, text: &str, _options: Option<crate::types::SynthesisOptions>) -> Result<Vec<u8>> {
        Err(openagentic_core::OpenAgenticError::NotImplemented(
            "Coqui TTS requires the Coqui TTS Python package to be installed. Run: pip install TTS".to_string()
        ))
    }

    async fn is_available(&self) -> bool {
        std::process::Command::new("tts")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn available_voices(&self) -> Vec<String> {
        vec!["tts_models/en/ljspeech/tacotron2-DDC".to_string()]
    }
}
