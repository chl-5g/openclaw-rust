//! User Info Step - 用户信息

use dialoguer::Select;
use crate::onboard::state::{WizardState, StepResult};

pub struct UserInfoStep;

impl UserInfoStep {
    pub fn new() -> Self {
        Self
    }

    pub fn run(state: &mut WizardState) -> anyhow::Result<StepResult> {
        println!("\n👤 请设置您的基本信息\n");

        // User name
        print!("您的名字 [User]: ");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut user_name = String::new();
        std::io::stdin().read_line(&mut user_name).ok();
        let user_name = user_name.trim().to_string();
        state.user_name = if user_name.is_empty() { "User".to_string() } else { user_name };

        // Language
        println!("\n🌐 选择界面语言:");
        let languages = vec!["English", "中文"];
        let lang_selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .items(&languages)
            .default(1)
            .interact()
            .unwrap_or(1);
        
        state.language = match lang_selection {
            0 => "en".to_string(),
            _ => "zh".to_string(),
        };

        println!("\n✅ 用户信息已设置");
        Ok(StepResult::success())
    }
}
