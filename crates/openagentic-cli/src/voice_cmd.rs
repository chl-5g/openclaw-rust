//! 语音配置 CLI 工具
//!
//! 提供命令行接口来管理语音功能和配置

use clap::Subcommand;
use openagentic_core::OpenAgenticError;
use openagentic_voice::{
    AudioPlayer, AudioUtils, SttProvider, SynthesisOptions, TalkModeBuilder, TalkModeEvent,
    TtsProvider, VoiceConfigManager,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use openagentic_voice::provider::ProviderRegistry;

lazy_static! {
    static ref PROVIDER_REGISTRY: Arc<RwLock<ProviderRegistry>> =
        Arc::new(RwLock::new(ProviderRegistry::new()));
}

#[derive(Debug, Subcommand)]
pub enum VoiceCommand {
    /// 设置语音 API Key
    SetKey {
        /// 提供商 (openai, azure, google)
        #[arg(default_value = "openai")]
        provider: String,
        /// API Key
        api_key: String,
        /// Base URL (可选)
        #[arg(short, long)]
        url: Option<String>,
    },

    /// 语音识别 (STT)
    Transcribe {
        /// 音频文件路径
        audio_file: String,
        /// 语言 (可选，自动检测)
        #[arg(short, long)]
        language: Option<String>,
        /// 提供商 (openai, local)
        #[arg(short, long, default_value = "openai")]
        provider: String,
    },

    /// 语音合成 (TTS)
    Synthesize {
        /// 要转换的文本
        text: String,
        /// 输出文件路径
        #[arg(short, long, default_value = "output.mp3")]
        output: String,
        /// 语音 (alloy, echo, fable, onyx, nova, shimmer)
        #[arg(short, long, default_value = "alloy")]
        voice: String,
        /// 语速 (0.25 - 4.0)
        #[arg(short, long, default_value = "1.0")]
        speed: f32,
        /// 提供商 (openai, edge)
        #[arg(short, long, default_value = "openai")]
        provider: String,
    },

    /// 启动持续对话模式
    Talk {
        /// 静音检测阈值
        #[arg(long, default_value = "0.02")]
        silence_threshold: f32,
        /// 静音超时 (毫秒)
        #[arg(long, default_value = "1500")]
        silence_timeout: u64,
        /// 是否自动继续
        #[arg(long, default_value = "true")]
        auto_continue: bool,
    },

    /// 启用/禁用语音功能
    Enable {
        /// 是否启用
        #[arg(default_value = "true")]
        enabled: bool,
    },

    /// 显示语音配置
    Config,

    /// 列出可用语音
    Voices {
        /// 提供商 (openai, edge)
        #[arg(default_value = "openai")]
        provider: String,
    },

    /// 检查麦克风
    CheckMic,

    /// 播放音频文件
    Play {
        /// 音频文件路径
        audio_file: String,
    },
}

impl VoiceCommand {
    /// 初始化全局提供商注册表
    pub fn init_registry(manager: &VoiceConfigManager) {
        if let Some(ref custom) = manager.voice.custom_providers {
            let registry = PROVIDER_REGISTRY.clone();
            tokio::runtime::Handle::current().block_on(async move {
                let reg = registry.write().await;
                reg.load_from_config(custom).await;
            });
        }
    }

    /// 执行命令
    pub async fn execute(&self) -> Result<(), OpenAgenticError> {
        let mut manager = VoiceConfigManager::load();

        match self {
            VoiceCommand::SetKey {
                provider,
                api_key,
                url,
            } => {
                let provider_lower = provider.to_lowercase();

                match provider_lower.as_str() {
                    "openai" => {
                        manager.set_stt_api_key(SttProvider::OpenAI, api_key.clone());
                        manager.set_tts_api_key(TtsProvider::OpenAI, api_key.clone());
                        if let Some(base_url) = url {
                            manager.set_openai_base_url(base_url.clone());
                        }
                        manager.save()?;
                        println!("✅ 已设置 OpenAI API Key");
                    }
                    "azure" => {
                        manager.set_stt_api_key(SttProvider::Azure, api_key.clone());
                        if let Some(base_url) = url {
                            manager.set_azure_region(base_url.clone());
                        }
                        manager.save()?;
                        println!("✅ 已设置 Azure Speech API Key");
                    }
                    "google" => {
                        manager.set_stt_api_key(SttProvider::Google, api_key.clone());
                        manager.save()?;
                        println!("✅ 已设置 Google Speech API Key");
                    }
                    _ => {
                        println!("❌ 不支持的提供商: {}", provider);
                        println!("\n支持的提供商: openai, azure, google");
                    }
                }
            }

            VoiceCommand::Transcribe {
                audio_file,
                language,
                provider,
            } => {
                let path = PathBuf::from(audio_file);
                if !path.exists() {
                    println!("❌ 文件不存在: {}", audio_file);
                    return Ok(());
                }

                println!("🔍 正在识别语音...");

                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => SttProvider::OpenAI,
                    "local" => SttProvider::LocalWhisper,
                    _ => SttProvider::OpenAI,
                };

                let config = manager.voice.stt_config.clone();
                let stt = openagentic_voice::create_stt(provider_type, config);

                match stt.transcribe_file(&path, language.as_deref()).await {
                    Ok(result) => {
                        println!("\n📝 转录结果:");
                        println!("{}", result.text);
                        if let Some(lang) = result.language {
                            println!("\n🌐 检测语言: {}", lang);
                        }
                        if let Some(duration) = result.duration {
                            println!("⏱️  时长: {:.2} 秒", duration);
                        }
                    }
                    Err(e) => {
                        println!("❌ 转录失败: {}", e);
                        println!("\n请确保已设置 API Key:");
                        println!("  open-agentic voice set-key openai sk-xxx");
                    }
                }
            }

            VoiceCommand::Synthesize {
                text,
                output,
                voice,
                speed,
                provider,
            } => {
                println!("🔊 正在合成语音...");

                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => TtsProvider::OpenAI,
                    "edge" => TtsProvider::Edge,
                    _ => TtsProvider::OpenAI,
                };

                let config = manager.voice.tts_config.clone();
                let tts = openagentic_voice::create_tts(provider_type, config);

                let options = SynthesisOptions {
                    voice: Some(voice.clone()),
                    speed: Some(*speed),
                    ..Default::default()
                };

                let output_path = PathBuf::from(output);

                match tts
                    .synthesize_to_file(text, &output_path, Some(options))
                    .await
                {
                    Ok(_) => {
                        println!("✅ 语音已保存到: {}", output);
                    }
                    Err(e) => {
                        println!("❌ 合成失败: {}", e);
                        println!("\n请确保已设置 API Key:");
                        println!("  open-agentic voice set-key openai sk-xxx");
                    }
                }
            }

            VoiceCommand::Talk {
                silence_threshold,
                silence_timeout,
                auto_continue,
            } => {
                println!("🎤 启动持续对话模式...");
                println!("   静音阈值: {}", silence_threshold);
                println!("   静音超时: {}ms", silence_timeout);
                println!("   自动继续: {}", auto_continue);
                println!();
                println!("按 Ctrl+C 退出");

                let talk_mode = TalkModeBuilder::new()
                    .silence_threshold(*silence_threshold)
                    .silence_timeout(*silence_timeout)
                    .auto_continue(*auto_continue)
                    .build();

                // 订阅事件
                let mut rx = talk_mode.subscribe();

                // 启动
                talk_mode.start().await?;

                // 监听事件
                loop {
                    match rx.recv().await {
                        Ok(event) => match event {
                            TalkModeEvent::ListeningStarted => {
                                println!("👂 正在监听...");
                            }
                            TalkModeEvent::Transcription(text) => {
                                println!("👤 你: {}", text);
                            }
                            TalkModeEvent::AiResponse(text) => {
                                println!("🤖 AI: {}", text);
                            }
                            TalkModeEvent::StateChanged(state) => {
                                tracing::debug!("状态: {:?}", state);
                            }
                            TalkModeEvent::Error(e) => {
                                println!("❌ 错误: {}", e);
                            }
                            _ => {}
                        },
                        Err(_) => break,
                    }

                    if !talk_mode.is_running().await {
                        break;
                    }
                }
            }

            VoiceCommand::Enable { enabled } => {
                manager.set_enabled(*enabled);
                manager.save()?;
                println!("✅ 语音功能已{}", if *enabled { "启用" } else { "禁用" });
            }

            VoiceCommand::Config => {
                println!("📋 语音配置:");
                println!();
                println!(
                    "  状态: {}",
                    if manager.voice.enabled {
                        "已启用"
                    } else {
                        "已禁用"
                    }
                );
                println!("  STT 提供商: {:?}", manager.voice.stt_provider);
                println!("  TTS 提供商: {:?}", manager.voice.tts_provider);
                println!();

                // STT 配置
                println!("  STT 配置:");
                if let Some(key) = &manager.voice.stt_config.openai_api_key {
                    let masked = mask_api_key(key);
                    println!("    OpenAI Key: {}", masked);
                } else {
                    println!("    OpenAI Key: 未设置");
                }
                if let Some(url) = &manager.voice.stt_config.openai_base_url {
                    println!("    Base URL: {}", url);
                }
                println!();

                // TTS 配置
                println!("  TTS 配置:");
                if let Some(key) = &manager.voice.tts_config.openai_api_key {
                    let masked = mask_api_key(key);
                    println!("    OpenAI Key: {}", masked);
                } else {
                    println!("    OpenAI Key: 未设置");
                }
                println!("    默认语音: {:?}", manager.voice.tts_config.default_voice);
                println!("    默认语速: {}", manager.voice.tts_config.default_speed);
            }

            VoiceCommand::Voices { provider } => {
                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => TtsProvider::OpenAI,
                    "edge" => TtsProvider::Edge,
                    _ => TtsProvider::OpenAI,
                };

                let config = manager.voice.tts_config.clone();
                let tts = openagentic_voice::create_tts(provider_type, config);
                let voices = tts.available_voices();

                println!("🎙️  可用语音 ({}) :", provider);
                println!();
                for voice in voices {
                    println!("  - {}", voice);
                }
            }

            VoiceCommand::CheckMic => {
                println!("🎤 检查麦克风...");

                match AudioUtils::get_input_device_info() {
                    Ok((name, info)) => {
                        println!("✅ 找到麦克风: {}", name);
                        println!("   - 采样率: {} Hz", info.sample_rate);
                        println!("   - 声道数: {}", info.channels);
                        println!("   - 位深度: {} bit", info.bits_per_sample);
                    }
                    Err(e) => {
                        println!("❌ 麦克风检测失败: {}", e);
                        println!();
                        println!("请检查:");
                        println!("  1. 系统已授权麦克风权限");
                        println!("  2. 麦克风已正确连接");
                    }
                }

                let input_devices = AudioUtils::list_input_devices().unwrap_or_default();
                if !input_devices.is_empty() {
                    println!();
                    println!("📋 输入设备列表:");
                    for (i, device) in input_devices.iter().enumerate() {
                        println!("   {}. {}", i + 1, device);
                    }
                }
            }

            VoiceCommand::Play { audio_file } => {
                let path = PathBuf::from(audio_file);
                if !path.exists() {
                    println!("❌ 文件不存在: {}", audio_file);
                    return Ok(());
                }

                println!("▶️  播放音频: {}", audio_file);

                let player = AudioPlayer::new();
                match player.play_file(&path) {
                    Ok(_) => {
                        println!("✅ 播放完成");
                    }
                    Err(e) => {
                        println!("❌ 播放失败: {}", e);
                        println!();
                        println!("尝试使用系统播放器...");
                        #[cfg(target_os = "macos")]
                        {
                            std::process::Command::new("open")
                                .arg(audio_file)
                                .spawn()
                                .ok();
                        }
                        #[cfg(target_os = "linux")]
                        {
                            std::process::Command::new("xdg-open")
                                .arg(audio_file)
                                .spawn()
                                .ok();
                        }
                        #[cfg(target_os = "windows")]
                        {
                            std::process::Command::new("start")
                                .arg("")
                                .arg(audio_file)
                                .spawn()
                                .ok();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// 隐藏 API Key 中间部分
fn mask_api_key(key: &str) -> String {
    if key.len() <= 12 {
        return "*".repeat(key.len());
    }

    let start = &key[..8];
    let end = &key[key.len() - 4..];
    format!("{}****{}", start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-short"), "********");
        assert_eq!(mask_api_key("sk-1234567890abcdef"), "sk-12345****cdef");
    }

    #[test]
    fn test_voice_command_parsing() {
        use clap::Parser;

        #[derive(Parser)]
        struct Cli {
            #[command(subcommand)]
            voice: VoiceCommand,
        }

        let check_mic = VoiceCommand::CheckMic;
        assert!(matches!(check_mic, VoiceCommand::CheckMic));

        let play = VoiceCommand::Play {
            audio_file: "test.mp3".to_string(),
        };
        assert!(matches!(play, VoiceCommand::Play { .. }));
    }
}
