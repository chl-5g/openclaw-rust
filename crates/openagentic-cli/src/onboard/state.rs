//! Wizard State - 向导状态管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardState {
    pub user_name: String,
    pub language: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub model: String,
    pub channels: Vec<String>,
    pub sandbox_enabled: bool,
    pub sandbox_type: String,
    pub dm_policy: String,
    pub voice_enabled: bool,
    pub browser_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub success: bool,
    pub error: Option<String>,
    pub skip_remaining: bool,
}

impl StepResult {
    pub fn success() -> Self {
        Self {
            success: true,
            error: None,
            skip_remaining: false,
        }
    }

    pub fn failure(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(msg.into()),
            skip_remaining: false,
        }
    }

    pub fn skip() -> Self {
        Self {
            success: true,
            error: None,
            skip_remaining: true,
        }
    }
}
