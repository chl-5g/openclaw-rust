//! 设置向导命令

use crate::onboard::wizard::OnboardWizard;

/// 运行设置向导
pub async fn run(quick: bool, force: bool) -> anyhow::Result<()> {
    let wizard = OnboardWizard::new(quick, force);
    wizard.run().await
}
