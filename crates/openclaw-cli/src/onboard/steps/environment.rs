//! Environment Step - 环境检测

use std::process::Command;
use crate::onboard::state::{WizardState, StepResult};
use crate::onboard::renderer::WizardRenderer;

pub struct EnvironmentStep;

impl EnvironmentStep {
    pub fn new() -> Self {
        Self
    }

    pub fn run(_state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n🔧 检查系统环境...");
        
        let mut has_error = false;
        let mut warnings = Vec::new();

        // Check Rust
        print!("   Rust... ");
        match Command::new("rustc").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✅ {}", version.trim());
            }
            _ => {
                println!("❌ 未安装");
                has_error = true;
            }
        }

        // Check Cargo
        print!("   Cargo... ");
        match Command::new("cargo").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✅ {}", version.trim());
            }
            _ => {
                println!("❌ 未安装");
                has_error = true;
            }
        }

        // Check Docker (optional)
        print!("   Docker (可选)... ");
        match Command::new("docker").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✅ {}", version.trim());
            }
            _ => {
                warnings.push("Docker 未安装 (如需 Docker 沙箱请安装)");
                println!("⚠️  未安装");
            }
        }

        // Check Node.js (optional)
        print!("   Node.js (可选)... ");
        match Command::new("node").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✅ {}", version.trim());
            }
            _ => {
                warnings.push("Node.js 未安装 (部分功能需要)");
                println!("⚠️  未安装");
            }
        }

        println!();
        if has_error {
            return Ok(StepResult::failure("环境检查失败，请先安装 Rust 和 Cargo"));
        }

        if warnings.is_empty() {
            println!("✅ 环境检查通过");
        } else {
            println!("⚠️  警告:");
            for w in &warnings {
                println!("   • {}", w);
            }
        }

        Ok(StepResult::success())
    }
}
