//! Security Step - 安全设置

use crate::onboard::state::{WizardState, StepResult};

pub struct SecurityStep;

impl SecurityStep {
    pub fn new() -> Self {
        Self
    }

    pub fn run(state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n🛡️ 安全设置\n");

        println!("  OpenAgentic 默认启用 WASM 沙箱保护您的数据。\n");

        // Sandbox type
        println!("📦 选择沙箱模式:");
        println!("  0. WASM (推荐 - 最安全)");
        println!("  1. Docker (需安装 Docker)");
        println!("  2. Native (无隔离 - 仅开发调试)");
        print!("请选择 [0]: ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim();
        
        match input {
            "1" => state.sandbox_type = "docker".to_string(),
            "2" => state.sandbox_type = "native".to_string(),
            _ => state.sandbox_type = "wasm".to_string(),
        }

        // Enable sandbox
        print!("启用安全沙箱? (Y/n): ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();
        
        state.sandbox_enabled = input.is_empty() || input == "y" || input == "yes";

        // DM Policy
        println!("\n📨 选择消息策略:");
        println!("  0. 配对模式 (推荐 - 需验证对方)");
        println!("  1. 开放模式 (接受所有消息)");
        println!("  2. 关闭 (不接受任何消息)");
        print!("请选择 [0]: ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim();
        
        match input {
            "1" => state.dm_policy = "open".to_string(),
            "2" => state.dm_policy = "closed".to_string(),
            _ => state.dm_policy = "pairing".to_string(),
        }

        // Voice (optional)
        print!("启用语音交互? 需要 STT/TTS 配置 (y/N): ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();
        
        state.voice_enabled = input == "y" || input == "yes";

        // Browser tool (optional)
        print!("启用浏览器控制工具? 需要 Chrome (y/N): ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();
        
        state.browser_enabled = input == "y" || input == "yes";

        println!("\n✅ 安全设置完成");
        println!("   沙箱: {} (已{})", 
            state.sandbox_type, 
            if state.sandbox_enabled { "启用" } else { "禁用" });
        
        Ok(StepResult::success())
    }
}
