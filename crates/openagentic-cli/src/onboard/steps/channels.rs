//! Channels Step - 通道配置

use dialoguer::MultiSelect;
use crate::onboard::state::{WizardState, StepResult};

pub struct ChannelsStep;

impl ChannelsStep {
    pub fn new() -> Self {
        Self
    }

    pub fn run(state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n💬 选择消息通道\n");
        
        println!("  选择您希望 AI 助手接收消息的平台。");
        println!("  您可以稍后通过命令添加更多通道。\n");

        let channel_names = vec![
            "Telegram",
            "Discord", 
            "WhatsApp",
            "Slack",
            "WebChat",
            "Signal",
            "Microsoft Teams",
            "iMessage",
            "Matrix",
            "Zalo",
        ];
        
        let selections = MultiSelect::new()
            .items(&channel_names)
            .with_prompt("选择通道 (空格键选择，回车确认)")
            .defaults(&[false; 10])
            .interact()
            .unwrap_or_default();

        let channels_map: Vec<(&str, &str)> = vec![
            ("Telegram", "Telegram Bot"),
            ("Discord", "Discord Bot"),
            ("WhatsApp", "WhatsApp Business"),
            ("Slack", "Slack App"),
            ("WebChat", "Web 在线聊天"),
            ("Signal", "Signal Messenger"),
            ("Microsoft Teams", "Microsoft Teams"),
            ("iMessage", "Apple iMessage (macOS)"),
            ("Matrix", "Matrix 协议"),
            ("Zalo", "Zalo (越南)"),
        ];

        for i in selections {
            state.channels.push(channels_map[i].0.to_string());
        }

        if state.channels.is_empty() {
            println!("\n⚠️  未选择任何通道，稍后可通过命令添加");
        } else {
            println!("\n✅ 已选择 {} 个通道:", state.channels.len());
            for ch in &state.channels {
                println!("   • {}", ch);
            }
        }

        Ok(StepResult::success())
    }
}
