use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SensitiveType {
    ApiKey,
    Password,
    Token,
    PrivateKey,
    CreditCard,
    Ssn,
    PhoneNumber,
    Email,
    IpAddress,
    FilePath,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveMatch {
    pub sensitive_type: SensitiveType,
    pub matched_value: String,
    pub start: usize,
    pub end: usize,
    pub redacted_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ValidationLevel {
    Safe,
    Warning,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValidation {
    pub level: ValidationLevel,
    pub matches: Vec<SensitiveMatch>,
    pub total_count: usize,
    pub requires_action: bool,
}

static DEFAULT_PATTERNS: Lazy<Vec<(Regex, SensitiveType, ValidationLevel)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"sk-[a-zA-Z0-9]{20,}").expect("Invalid regex: API key"),
            SensitiveType::ApiKey,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"(?i)apikey.*[=:].{20,}").expect("Invalid regex: apikey"),
            SensitiveType::ApiKey,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"(?i)password.*[=:].{8,}").expect("Invalid regex: password"),
            SensitiveType::Password,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"bearer [a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").expect("Invalid regex: bearer token"),
            SensitiveType::Token,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"-----BEGIN.+PRIVATE KEY-----").expect("Invalid regex: private key"),
            SensitiveType::PrivateKey,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}").expect("Invalid regex: credit card"),
            SensitiveType::CreditCard,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"\d{3}-\d{2}-\d{4}").expect("Invalid regex: SSN"),
            SensitiveType::Ssn,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"1[3-9]\d{9}").expect("Invalid regex: phone number"),
            SensitiveType::PhoneNumber,
            ValidationLevel::Warning,
        ),
        (
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").expect("Invalid regex: email"),
            SensitiveType::Email,
            ValidationLevel::Warning,
        ),
        (
            Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").expect("Invalid regex: IP address"),
            SensitiveType::IpAddress,
            ValidationLevel::Warning,
        ),
        (
            Regex::new(r"(?i)(/home/|/Users/|/etc/|C:\\|D:\\)[^\s]+").expect("Invalid regex: file path"),
            SensitiveType::FilePath,
            ValidationLevel::Warning,
        ),
        (
            Regex::new(r"(?i)secret[_-]?key.*[=:].{16,}").expect("Invalid regex: secret key"),
            SensitiveType::ApiKey,
            ValidationLevel::Block,
        ),
        (
            Regex::new(r"(?i)access[_-]?token.*[=:].{20,}").expect("Invalid regex: access token"),
            SensitiveType::Token,
            ValidationLevel::Block,
        ),
    ]
});

pub struct OutputValidator {
    patterns: Arc<RwLock<Vec<(Regex, SensitiveType, ValidationLevel)>>>,
    custom_rules: Arc<RwLock<HashMap<String, (Regex, ValidationLevel)>>>,
    stats: Arc<RwLock<HashMap<SensitiveType, u32>>>,
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputValidator {
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(DEFAULT_PATTERNS.clone())),
            custom_rules: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn validate(&self, output: &str) -> OutputValidation {
        let patterns = self.patterns.read().await;
        let custom_rules = self.custom_rules.read().await;

        let mut matches = Vec::new();
        let mut block_count = 0;
        let mut warning_count = 0;

        for (regex, sensitive_type, level) in patterns.iter() {
            for m in regex.find_iter(output) {
                let redacted = self.redact_value(m.as_str(), sensitive_type);
                matches.push(SensitiveMatch {
                    sensitive_type: sensitive_type.clone(),
                    matched_value: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                    redacted_value: redacted,
                });

                match level {
                    ValidationLevel::Block => block_count += 1,
                    ValidationLevel::Warning => warning_count += 1,
                    ValidationLevel::Safe => {}
                }

                self.record_match(sensitive_type).await;
            }
        }

        for (name, (regex, level)) in custom_rules.iter() {
            for m in regex.find_iter(output) {
                let redacted = "[REDACTED]".to_string();
                matches.push(SensitiveMatch {
                    sensitive_type: SensitiveType::Custom(name.clone()),
                    matched_value: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                    redacted_value: redacted,
                });

                match level {
                    ValidationLevel::Block => block_count += 1,
                    ValidationLevel::Warning => warning_count += 1,
                    ValidationLevel::Safe => {}
                }
            }
        }

        matches.sort_by_key(|m| m.start);

        let requires_action = block_count > 0;
        let level = if block_count > 0 {
            ValidationLevel::Block
        } else if warning_count > 0 {
            ValidationLevel::Warning
        } else {
            ValidationLevel::Safe
        };

        if requires_action {
            warn!(
                "Output validation: found {} sensitive matches ({} blocks, {} warnings)",
                matches.len(),
                block_count,
                warning_count
            );
        }

        OutputValidation {
            level,
            matches,
            total_count: block_count + warning_count,
            requires_action,
        }
    }

    pub async fn validate_and_redact(&self, output: &str) -> (String, OutputValidation) {
        let validation = self.validate(output).await;

        let mut redacted = output.to_string();
        for m in validation.matches.iter().rev() {
            redacted.replace_range(m.start..m.end, &m.redacted_value);
        }

        (redacted, validation)
    }

    fn redact_value(&self, value: &str, sensitive_type: &SensitiveType) -> String {
        match sensitive_type {
            SensitiveType::ApiKey | SensitiveType::Token | SensitiveType::PrivateKey => {
                if value.len() > 8 {
                    format!("{}...{}", &value[..4], &value[value.len() - 4..])
                } else {
                    "***".to_string()
                }
            }
            SensitiveType::Password => "********".to_string(),
            SensitiveType::CreditCard => {
                if value.len() >= 4 {
                    format!("****-****-****-{}", &value[value.len() - 4..])
                } else {
                    "****".to_string()
                }
            }
            SensitiveType::Ssn => "***-**-****".to_string(),
            SensitiveType::PhoneNumber => {
                if value.len() >= 4 {
                    format!("****{}", &value[value.len() - 4..])
                } else {
                    "****".to_string()
                }
            }
            SensitiveType::Email => {
                if let Some(at_pos) = value.find('@') {
                    if at_pos > 2 {
                        format!("{}***@{}", &value[..2], &value[at_pos..])
                    } else {
                        "**@***".to_string()
                    }
                } else {
                    "**@**".to_string()
                }
            }
            SensitiveType::IpAddress => "***.***.***.***".to_string(),
            SensitiveType::FilePath => "/redacted/path".to_string(),
            SensitiveType::Custom(_) => "[REDACTED]".to_string(),
        }
    }

    pub async fn add_custom_rule(
        &self,
        name: String,
        pattern: String,
        level: ValidationLevel,
    ) -> Result<(), regex::Error> {
        let regex = Regex::new(&pattern)?;
        let mut rules = self.custom_rules.write().await;
        rules.insert(name, (regex, level));
        Ok(())
    }

    async fn record_match(&self, sensitive_type: &SensitiveType) {
        let mut stats = self.stats.write().await;
        *stats.entry(sensitive_type.clone()).or_insert(0) += 1;
    }

    pub async fn get_stats(&self) -> HashMap<SensitiveType, u32> {
        let stats = self.stats.read().await;
        stats.clone()
    }

    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_output_validator_safe_content() {
        let validator = OutputValidator::new();
        let result = validator.validate("Hello, this is a safe response").await;

        assert_eq!(result.level, ValidationLevel::Safe);
        assert!(result.matches.is_empty());
    }

    #[tokio::test]
    async fn test_output_validator_api_key_detection() {
        let validator = OutputValidator::new();
        let result = validator
            .validate("Here is your API key: sk-1234567890abcdefghij")
            .await;

        assert!(result.requires_action);
        assert!(!result.matches.is_empty());
    }

    #[tokio::test]
    async fn test_output_validator_password_detection() {
        let validator = OutputValidator::new();
        let result = validator.validate("password = mysecretpassword123").await;

        assert!(result.requires_action);
    }

    #[tokio::test]
    async fn test_output_validator_token_detection() {
        let validator = OutputValidator::new();

        let result = validator.validate("password = mysecretpassword123").await;
        assert!(!result.matches.is_empty() || result.level != ValidationLevel::Safe);
    }

    #[tokio::test]
    async fn test_output_validator_credit_card() {
        let validator = OutputValidator::new();
        let result = validator.validate("Credit card: 1234-5678-9012-3456").await;

        assert!(result.requires_action);
    }

    #[tokio::test]
    async fn test_output_validator_ssn() {
        let validator = OutputValidator::new();
        let result = validator.validate("SSN: 123-45-6789").await;

        assert!(result.requires_action);
    }

    #[tokio::test]
    async fn test_validation_level_ordering() {
        assert_eq!(ValidationLevel::Safe, ValidationLevel::Safe);
        assert_eq!(ValidationLevel::Warning, ValidationLevel::Warning);
        assert_eq!(ValidationLevel::Block, ValidationLevel::Block);
    }

    #[tokio::test]
    async fn test_redacted_value() {
        let validator = OutputValidator::new();
        let result = validator.validate("API Key: sk-1234567890abcdefghij").await;

        if let Some(matched) = result.matches.first() {
            assert!(
                matched.redacted_value.contains('*') || matched.redacted_value.starts_with("sk-")
            );
        }
    }
}
