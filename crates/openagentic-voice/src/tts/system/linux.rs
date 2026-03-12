//! Linux System TTS using eSpeak or Festival

use crate::types::{TtsConfig, TtsProvider};
use async_trait::async_trait;
use openagentic_core::Result;
use std::process::Command;

pub struct LinuxSystemTts {
    voice: String,
    rate: i32,
}

impl LinuxSystemTts {
    pub fn new(config: TtsConfig) -> Self {
        let voice = if config.default_voice.as_str().is_empty() {
            "en".to_string()
        } else {
            config.default_voice.as_str().to_string()
        };
        let rate = (config.default_speed * 150.0) as i32;

        Self { voice, rate }
    }

    fn get_command(&self) -> (&str, Vec<String>) {
        if Command::new("espeak").arg("--version").output().is_ok() {
            ("espeak", vec!["-w".into(), "/tmp/openagentic_tts.wav".into(), "-s".into(), self.rate.to_string(), "-v".into(), self.voice.clone()])
        } else if Command::new("festival").arg("--version").output().is_ok() {
            ("festival", vec!["--tts".into()])
        } else {
            ("espeak", vec!["-w".into(), "/tmp/openagentic_tts.wav".into(), "-s".into(), "150".into()])
        }
    }
}

#[async_trait]
impl crate::tts::TextToSpeech for LinuxSystemTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::LinuxSystem
    }

    async fn synthesize(&self, text: &str, _options: Option<crate::types::SynthesisOptions>) -> Result<Vec<u8>> {
        let (cmd, _args) = self.get_command();

        if cmd == "espeak" {
            let mut cmd = Command::new("espeak");
            cmd.args(["-w", "/tmp/openagentic_tts.wav", "-s", &self.rate.to_string(), "-v", &self.voice])
                .arg(text);

            let output = cmd.output().map_err(|e| openagentic_core::OpenAgenticError::Io(e))?;

            if !output.status.success() {
                return Err(openagentic_core::OpenAgenticError::Io(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("espeak command failed: {}", String::from_utf8_lossy(&output.stderr)),
                    ),
                ));
            }

            let audio_data = std::fs::read("/tmp/openagentic_tts.wav")
                .map_err(|e| openagentic_core::OpenAgenticError::Io(e))?;

            std::fs::remove_file("/tmp/openagentic_tts.wav").ok();

            Ok(audio_data)
        } else {
            Err(openagentic_core::OpenAgenticError::Io(
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No TTS engine found. Please install espeak or festival.",
                ),
            ))
        }
    }

    async fn is_available(&self) -> bool {
        Command::new("espeak").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
            || Command::new("festival").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn available_voices(&self) -> Vec<String> {
        let output = Command::new("espeak")
            .arg("--voices")
            .output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .skip(1)
                    .filter_map(|line| line.split_whitespace().nth(1).map(String::from))
                    .collect()
            }
            _ => vec![
                "en".to_string(),
                "en-us".to_string(),
                "en-gb".to_string(),
                "en-sc".to_string(),
            ],
        }
    }
}
