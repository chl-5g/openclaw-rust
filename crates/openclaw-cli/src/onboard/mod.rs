//! Onboard 向导模块

pub mod state;
pub mod steps;
pub mod renderer;

pub use state::{WizardState, StepResult};

pub mod wizard {
    use anyhow::Result;
    use std::string::ToString;
    use super::state::{WizardState, StepResult};
    use super::steps::{EnvironmentStep, UserInfoStep, ProviderStep, ChannelsStep, SecurityStep, StartStep};

    pub struct OnboardWizard {
        quick: bool,
        force: bool,
    }

    impl OnboardWizard {
        pub fn new(quick: bool, force: bool) -> Self {
            Self { quick, force }
        }

        pub async fn run(&self) -> Result<()> {
            let mut state = WizardState::default();

            let steps: Vec<(&str, &str, fn(&mut WizardState) -> Result<StepResult>)> = vec![
                ("环境检测", "检查系统环境是否满足运行要求", EnvironmentStep::run),
                ("用户信息", "设置您的基本信息", UserInfoStep::run),
                ("AI 提供商", "选择 AI 模型提供商并配置 API Key", ProviderStep::run),
                ("消息通道", "选择要启用的消息通道", ChannelsStep::run),
                ("安全设置", "配置沙箱隔离和消息策略", SecurityStep::run),
            ];

            if !self.quick {
                for (i, (name, _desc, func)) in steps.iter().enumerate() {
                    super::renderer::WizardRenderer::show_progress(i + 1, steps.len());
                    println!("\n📋 步骤 {}: {}", i + 1, name);
                    println!("{}", "-".repeat(40));

                    match func(&mut state) {
                        Ok(result) => {
                            if !result.success {
                                println!("❌ 步骤失败: {}", result.error.unwrap_or_default());
                                return Ok(());
                            }
                            if result.skip_remaining {
                                println!("⏭️  跳过剩余步骤");
                                break;
                            }
                        }
                        Err(e) => {
                            println!("❌ 错误: {}", e);
                            return Err(e);
                        }
                    }
                }
            }

            // 最后一步：保存并启动
            println!("\n📋 步骤 {}: 完成", steps.len() + 1);
            println!("{}", "-".repeat(40));
            StartStep::run(&mut state).await?;

            println!("\n🎉 初始化完成！");
            println!("运行 `openclaw-rust gateway` 启动服务");

            Ok(())
        }
    }
}
