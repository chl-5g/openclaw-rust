//! Wizard Renderer - 向导 UI 渲染

pub struct WizardRenderer;

impl WizardRenderer {
    pub fn show_header(title: &str) {
        println!("\n{}", "═".repeat(50));
        println!("  🦞 {}", title);
        println!("{}", "═".repeat(50));
    }

    pub fn show_welcome() {
        Self::show_header("OpenClaw 设置向导");
        println!();
        println!("  欢迎使用 OpenClaw！", );
        println!("  本向导将帮助您完成初始配置。");
        println!();
        println!("  按 Ctrl+C 可随时退出");
        println!();
    }

    pub fn show_progress(current: usize, total: usize) {
        let percent = current as f32 / total as f32 * 100.0;
        println!("\n📍 进度: [{}/{}] {:.0}%", current, total, percent);
    }

    pub fn show_step(name: &str, description: &str) {
        println!("\n▶ {}", name);
        println!("  {}", description);
    }

    pub fn show_success(message: &str) {
        println!("\n✅ {}", message);
    }

    pub fn show_error(message: &str) {
        println!("\n❌ 错误: {}", message);
    }

    pub fn show_warning(message: &str) {
        println!("\n⚠️  警告: {}", message);
    }

    pub fn show_info(message: &str) {
        println!("\nℹ️  {}", message);
    }

    pub fn separator() {
        println!("\n{}", "─".repeat(50));
    }

    pub fn press_enter() {
        println!("\n[按回车键继续...]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
    }
}
