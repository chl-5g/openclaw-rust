//! Skill Validator - 技能验证器
//!
//! 验证生成技能的质量和安全

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub status: ValidationStatus,
    pub message: String,
    pub warnings: Vec<String>,
    pub details: Vec<ValidationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    Approved,
    Rejected,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    pub rule: String,
    pub passed: bool,
    pub message: String,
}

pub struct SkillValidator {
    config: ValidatorConfig,
    dangerous_patterns: Vec<Regex>,
}

#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    pub max_function_lines: usize,
    pub max_loop_count: usize,
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub allow_shell: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            max_function_lines: 100,
            max_loop_count: 10,
            allow_network: false,
            allow_filesystem: false,
            allow_shell: false,
        }
    }
}

impl Default for SkillValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillValidator {
    pub fn new() -> Self {
        let dangerous_patterns = vec![
            Regex::new(r"std::fs::remove").unwrap(),
            Regex::new(r"std::fs::remove_dir_all").unwrap(),
            Regex::new(r"std::process::Command::new\s*\(").unwrap(),
            Regex::new(r"\.exec\s*\(").unwrap(),
            Regex::new(r"\.system\s*\(").unwrap(),
            Regex::new(r"eval\s*\(").unwrap(),
            Regex::new(r"unsafe\s*\{").unwrap(),
        ];

        Self {
            config: ValidatorConfig::default(),
            dangerous_patterns,
        }
    }

    pub fn with_config(config: ValidatorConfig) -> Self {
        Self {
            config,
            dangerous_patterns: vec![
                Regex::new(r"std::fs::remove").unwrap(),
                Regex::new(r"std::fs::remove_dir_all").unwrap(),
                Regex::new(r"std::process::Command::new\s*\(").unwrap(),
                Regex::new(r"\.exec\s*\(").unwrap(),
                Regex::new(r"\.system\s*\(").unwrap(),
                Regex::new(r"eval\s*\(").unwrap(),
                Regex::new(r"unsafe\s*\{").unwrap(),
            ],
        }
    }

    pub fn validate(&self, code: &str) -> ValidationResult {
        let mut warnings = Vec::new();
        let mut details = Vec::new();

        let detail = self.check_dangerous_operations(code);
        let passed = detail.passed.clone();
        let msg = detail.message.clone();
        details.push(detail);
        if !passed {
            warnings.push(msg);
        }

        let detail = self.check_complexity(code);
        details.push(detail);

        let detail = self.check_loop_count(code);
        details.push(detail);

        let detail = self.check_error_handling(code);
        details.push(detail);

        let detail = self.check_async_usage(code);
        details.push(detail);

        let rejected = details.iter().any(|d| !d.passed && d.rule == "dangerous_operations");
        let needs_review = details.iter().any(|d| !d.passed && d.rule != "dangerous_operations");

        let status = if rejected {
            ValidationStatus::Rejected
        } else if needs_review {
            ValidationStatus::NeedsReview
        } else {
            ValidationStatus::Approved
        };

        let message = match status {
            ValidationStatus::Approved => "Skill validation passed".to_string(),
            ValidationStatus::Rejected => "Skill validation failed".to_string(),
            ValidationStatus::NeedsReview => "Skill needs manual review".to_string(),
        };

        ValidationResult {
            status,
            message,
            warnings,
            details,
        }
    }

    fn check_dangerous_operations(&self, code: &str) -> ValidationDetail {
        let mut found_dangerous = Vec::new();

        for pattern in &self.dangerous_patterns {
            if pattern.is_match(code) {
                found_dangerous.push(pattern.to_string());
            }
        }

        if found_dangerous.is_empty() {
            ValidationDetail {
                rule: "dangerous_operations".to_string(),
                passed: true,
                message: "No dangerous operations detected".to_string(),
            }
        } else {
            ValidationDetail {
                rule: "dangerous_operations".to_string(),
                passed: false,
                message: format!("Dangerous patterns detected: {}", found_dangerous.join(", ")),
            }
        }
    }

    fn check_complexity(&self, code: &str) -> ValidationDetail {
        let lines: Vec<&str> = code.lines().collect();
        let line_count = lines.len();

        if line_count > self.config.max_function_lines {
            ValidationDetail {
                rule: "complexity".to_string(),
                passed: false,
                message: format!(
                    "Function too complex: {} lines, max allowed {}",
                    line_count,
                    self.config.max_function_lines
                ),
            }
        } else {
            ValidationDetail {
                rule: "complexity".to_string(),
                passed: true,
                message: "Code complexity is acceptable".to_string(),
            }
        }
    }

    fn check_loop_count(&self, code: &str) -> ValidationDetail {
        let for_loops = code.matches("for ").count();
        let while_loops = code.matches("while ").count();
        let loop_count = for_loops + while_loops;

        if loop_count > self.config.max_loop_count {
            ValidationDetail {
                rule: "loop_count".to_string(),
                passed: false,
                message: format!(
                    "Too many loops: {}, max allowed {}",
                    loop_count, self.config.max_loop_count
                ),
            }
        } else {
            ValidationDetail {
                rule: "loop_count".to_string(),
                passed: true,
                message: "Loop count is acceptable".to_string(),
            }
        }
    }

    fn check_error_handling(&self, code: &str) -> ValidationDetail {
        let has_result = code.contains("Result<");
        let has_option = code.contains("Option<");
        let has_question_mark = code.contains('?');
        let has_if_let = code.contains("if let");
        let has_match = code.contains("match ");

        let has_error_handling = has_result || has_option || has_question_mark || has_if_let || has_match;

        if has_error_handling {
            ValidationDetail {
                rule: "error_handling".to_string(),
                passed: true,
                message: "Proper error handling detected".to_string(),
            }
        } else {
            ValidationDetail {
                rule: "error_handling".to_string(),
                passed: false,
                message: "No error handling detected".to_string(),
            }
        }
    }

    fn check_async_usage(&self, code: &str) -> ValidationDetail {
        let has_async_fn = code.contains("async fn");
        let has_await = code.contains(".await");

        if has_async_fn || has_await {
            ValidationDetail {
                rule: "async_usage".to_string(),
                passed: true,
                message: "Proper async/await usage".to_string(),
            }
        } else {
            ValidationDetail {
                rule: "async_usage".to_string(),
                passed: false,
                message: "No async/await usage detected".to_string(),
            }
        }
    }

    pub fn validate_tool_sequence(&self, tool_calls: &[crate::evo::pattern_analyzer::ToolCall]) -> ValidationResult {
        let mut details = Vec::new();
        
        let tool_count = tool_calls.len();
        let passed = tool_count <= 20;
        details.push(ValidationDetail {
            rule: "tool_count".to_string(),
            passed,
            message: if passed {
                format!("Tool call count {} is acceptable", tool_count)
            } else {
                format!("Too many tool calls: {}", tool_count)
            },
        });
        
        let tool_names: Vec<_> = tool_calls.iter().map(|c| c.name.clone()).collect();
        let unique_names: HashSet<_> = tool_names.iter().collect();
        let has_duplicates = unique_names.len() < tool_names.len();
        details.push(ValidationDetail {
            rule: "duplicate_calls".to_string(),
            passed: !has_duplicates,
            message: if has_duplicates {
                "Duplicate tool calls detected".to_string()
            } else {
                "No duplicate tool calls".to_string()
            },
        });
        
        let failed_calls = tool_calls.iter().filter(|c| c.result.is_none()).count();
        let failure_rate = if !tool_calls.is_empty() {
            failed_calls as f64 / tool_calls.len() as f64
        } else {
            0.0
        };
        
        let passed = failure_rate < 0.3;
        details.push(ValidationDetail {
            rule: "failure_rate".to_string(),
            passed,
            message: format!("Failure rate: {:.1}%", failure_rate * 100.0),
        });
        
        let rejected = details.iter().any(|d| !d.passed && d.rule == "tool_count");
        
        ValidationResult {
            status: if rejected { ValidationStatus::Rejected } else { ValidationStatus::Approved },
            message: "Tool sequence validation completed".to_string(),
            warnings: vec![],
            details,
        }
    }

    pub fn validate_pattern_reusability(&self, pattern: &crate::evo::pattern_analyzer::TaskPattern) -> ValidationResult {
        let mut details = Vec::new();
        
        let tool_count = pattern.tool_sequence.len();
        let passed = tool_count >= 2 && tool_count <= 10;
        details.push(ValidationDetail {
            rule: "pattern_length".to_string(),
            passed,
            message: format!("Tool sequence length: {}", tool_count),
        });
        
        let total_params = pattern.param_patterns.len();
        let generic_params = pattern.param_patterns.iter().filter(|p| p.is_generic).count();
        let generic_ratio = if total_params > 0 {
            generic_params as f64 / total_params as f64
        } else {
            0.0
        };
        
        let passed = generic_ratio >= 0.3;
        details.push(ValidationDetail {
            rule: "parameter_generic_ratio".to_string(),
            passed,
            message: format!("Generic parameter ratio: {:.1}%", generic_ratio * 100.0),
        });
        
        let passed = pattern.reusability_score >= 0.5;
        details.push(ValidationDetail {
            rule: "reusability_score".to_string(),
            passed,
            message: format!("Reusability score: {:.1}%", pattern.reusability_score * 100.0),
        });
        
        let status = if details.iter().all(|d| d.passed) {
            ValidationStatus::Approved
        } else if details.iter().any(|d| !d.passed && d.rule == "pattern_length") {
            ValidationStatus::Rejected
        } else {
            ValidationStatus::NeedsReview
        };
        
        ValidationResult {
            status,
            message: "Pattern reusability validation completed".to_string(),
            warnings: vec![],
            details,
        }
    }

    pub fn validate_execution_time(&self, duration_ms: u64, expected_duration_ms: u64) -> ValidationResult {
        let mut details = Vec::new();
        
        let passed = duration_ms <= expected_duration_ms * 2;
        details.push(ValidationDetail {
            rule: "execution_time".to_string(),
            passed,
            message: if passed {
                format!("Execution time {}ms is acceptable", duration_ms)
            } else {
                format!("Execution time {}ms exceeds limit {}ms", duration_ms, expected_duration_ms * 2)
            },
        });
        
        ValidationResult {
            status: if passed { ValidationStatus::Approved } else { ValidationStatus::NeedsReview },
            message: "Execution time validation completed".to_string(),
            warnings: vec![],
            details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evo::pattern_analyzer::{ExecutionStep, ParamPattern, ParamType, TaskPattern, ToolCall, ToolCallPattern};

    #[test]
    fn test_validate_safe_code() {
        let validator = SkillValidator::new();

        let code = r#"
async fn fetch_data(url: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await
        .map_err(|e| e.to_string())?;
    Ok(response.text().await.map_err(|e| e.to_string())?)
}
"#;

        let result = validator.validate(code);

        assert_eq!(result.status, ValidationStatus::Approved);
    }

    #[test]
    fn test_validate_dangerous_code() {
        let validator = SkillValidator::new();

        let code = r#"
fn delete_file(path: String) {
    std::fs::remove_file(path).unwrap();
}
"#;

        let result = validator.validate(code);

        assert_eq!(result.status, ValidationStatus::Rejected);
    }

    #[test]
    fn test_validate_shell_code() {
        let validator = SkillValidator::new();

        let code = r#"
fn run_command(cmd: String) {
    std::process::Command::new("sh").arg("-c").arg(cmd).spawn().unwrap();
}
"#;

        let result = validator.validate(code);

        assert_eq!(result.status, ValidationStatus::Rejected);
    }

    #[test]
    fn test_validate_complex_code() {
        let validator = SkillValidator::new();

        let code = std::iter::repeat("let x = 1;\n")
            .take(150)
            .collect::<String>();

        let result = validator.validate(&code);

        let complexity = result.details.iter().find(|d| d.rule == "complexity");
        assert!(complexity.is_some());
        assert!(!complexity.unwrap().passed);
    }

    #[test]
    fn test_validate_many_loops() {
        let validator = SkillValidator::new();

        let code = r#"
fn process() {
    let mut count = 0;
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
    for i in 0..11 { count += i; }
}
"#;

        let result = validator.validate(code);

        let loop_detail = result.details.iter().find(|d| d.rule == "loop_count");
        assert!(loop_detail.is_some());
        assert!(!loop_detail.unwrap().passed);
    }

    #[test]
    fn test_validate_no_error_handling() {
        let validator = SkillValidator::new();

        let code = r#"
fn fetch(url: String) -> String {
    reqwest::blocking::get(url).unwrap().text().unwrap()
}
"#;

        let result = validator.validate(code);

        assert!(!result.details.iter().find(|d| d.rule == "error_handling").unwrap().passed);
    }

    #[test]
    fn test_validate_with_config() {
        let config = ValidatorConfig {
            max_function_lines: 50,
            max_loop_count: 5,
            allow_network: true,
            allow_filesystem: true,
            allow_shell: true,
        };

        let validator = SkillValidator::with_config(config);

        assert_eq!(validator.config.max_function_lines, 50);
        assert_eq!(validator.config.max_loop_count, 5);
    }

    #[test]
    fn test_validate_sync_code() {
        let validator = SkillValidator::new();

        let code = r#"
fn fetch_sync(url: String) -> Result<String, String> {
    Ok("result".to_string())
}
"#;

        let result = validator.validate(code);

        let async_detail = result.details.iter().find(|d| d.rule == "async_usage");
        assert!(async_detail.is_some());
        assert!(!async_detail.unwrap().passed);
    }

    #[test]
    fn test_validate_empty_code() {
        let validator = SkillValidator::new();

        let result = validator.validate("");

        assert_eq!(result.status, ValidationStatus::NeedsReview);
    }

    #[test]
    fn test_multiple_warnings() {
        let validator = SkillValidator::new();

        let code = r#"
fn fetch(url: String) -> String {
    std::process::Command::new("ls").spawn().unwrap();
    for i in 0..1000 { println!("{}", i); }
    reqwest::blocking::get(url).unwrap()
}
"#;

        let result = validator.validate(code);

        assert!(result.status == ValidationStatus::Rejected || result.status == ValidationStatus::NeedsReview);
    }

    #[test]
    fn test_validate_tool_sequence() {
        let validator = SkillValidator::new();
        
        let tool_calls = vec![
            crate::evo::pattern_analyzer::ToolCall {
                name: "fetch".to_string(),
                arguments: serde_json::json!({}),
                result: Some(serde_json::json!("data")),
                duration_ms: 100,
            },
            ToolCall {
                name: "parse".to_string(),
                arguments: serde_json::json!({}),
                result: Some(serde_json::json!("parsed")),
                duration_ms: 50,
            },
        ];
        
        let result = validator.validate_tool_sequence(&tool_calls);
        assert_eq!(result.status, ValidationStatus::Approved);
        assert!(result.details.iter().any(|d| d.rule == "tool_count" && d.passed));
    }

    #[test]
    fn test_validate_tool_sequence_too_many() {
        let validator = SkillValidator::new();
        
        let tool_calls: Vec<_> = (0..25).map(|i| {
            ToolCall {
                name: format!("tool_{}", i),
                arguments: serde_json::json!({}),
                result: Some(serde_json::json!("data")),
                duration_ms: 100,
            }
        }).collect();
        
        let result = validator.validate_tool_sequence(&tool_calls);
        assert!(result.status == ValidationStatus::Rejected || result.status == ValidationStatus::NeedsReview);
    }

    #[test]
    fn test_validate_pattern_reusability() {
        let validator = SkillValidator::new();
        
        let pattern = TaskPattern {
            id: "test-1".to_string(),
            task_category: "search".to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "search".to_string(),
                    param_schema: HashMap::new(),
                    result_schema: HashMap::new(),
                },
                ToolCallPattern {
                    tool_name: "fetch".to_string(),
                    param_schema: HashMap::new(),
                    result_schema: HashMap::new(),
                },
            ],
            param_patterns: vec![
                ParamPattern {
                    name: "query".to_string(),
                    param_type: ParamType::String,
                    is_generic: true,
                    examples: vec!["<QUERY>".to_string()],
                },
            ],
            success_indicators: vec![],
            steps: vec![
                ExecutionStep {
                    step_number: 1,
                    tool_name: "search".to_string(),
                    input_summary: "query".to_string(),
                    output_summary: "found".to_string(),
                    success: true,
                },
            ],
            reusability_score: 0.8,
            source_task_id: "task-1".to_string(),
            created_at: chrono::Utc::now(),
        };
        
        let result = validator.validate_pattern_reusability(&pattern);
        assert!(result.details.iter().any(|d| d.rule == "pattern_length" && d.passed));
    }

    #[test]
    fn test_validate_execution_time() {
        let validator = SkillValidator::new();
        
        let result = validator.validate_execution_time(100, 1000);
        assert_eq!(result.status, ValidationStatus::Approved);
        
        let result = validator.validate_execution_time(2500, 1000);
        assert_eq!(result.status, ValidationStatus::NeedsReview);
    }
}
