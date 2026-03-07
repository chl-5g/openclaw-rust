//! OpenClaw Core - 核心类型和抽象
//!
//! 提供项目的基础类型、错误处理、配置等核心功能。

pub mod config;
pub mod config_loader;
pub mod error;
pub mod group_context;
pub mod i18n;
pub mod message;
pub mod session;
pub mod user_config;

pub use error::{OpenClawError, Result};
pub use message::{Message, Content, Role};
pub use config::Config;
pub use session::Session;
pub use user_config::{UserConfig, UserConfigManager, UserProviderConfig};
pub use i18n::{Locale, I18n};
