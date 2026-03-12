//! Pattern Analyzer - 任务模式分析器
//! 
//! 从任务执行中提取可复用的模式，用于技能进化学习

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPattern {
    pub id: String,
    pub task_category: String,
    pub tool_sequence: Vec<ToolCallPattern>,
    pub param_patterns: Vec<ParamPattern>,
    pub success_indicators: Vec<String>,
    pub steps: Vec<ExecutionStep>,
    pub reusability_score: f64,
    pub source_task_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPattern {
    pub tool_name: String,
    pub param_schema: HashMap<String, ParamType>,
    pub result_schema: HashMap<String, ParamType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamPattern {
    pub name: String,
    pub param_type: ParamType,
    pub is_generic: bool,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Unknown,
}

impl ParamType {
    pub fn from_json_value(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(_) => ParamType::String,
            serde_json::Value::Number(_) => ParamType::Number,
            serde_json::Value::Bool(_) => ParamType::Boolean,
            serde_json::Value::Object(_) => ParamType::Object,
            serde_json::Value::Array(_) => ParamType::Array,
            _ => ParamType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_number: u32,
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub task_type: String,
    pub input: String,
    pub tool_calls: Vec<ToolCall>,
    pub success: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_id: String,
    pub similarity: f64,
    pub match_type: MatchType,
    pub differences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    Exact,
    Partial,
    Potential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCluster {
    pub id: String,
    pub patterns: Vec<String>,
    pub frequency: u32,
    pub avg_reusability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub score: f64,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub rule: String,
    pub severity: IssueSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct PatternAnalyzer {
    config: AnalyzerConfig,
    generalization_patterns: Vec<(Regex, String)>,
}

#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub min_reusability_threshold: f64,
    pub max_tool_sequence_length: usize,
    pub enable_deep_analysis: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            min_reusability_threshold: 0.5,
            max_tool_sequence_length: 20,
            enable_deep_analysis: true,
        }
    }
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternAnalyzer {
    pub fn new() -> Self {
        Self {
            config: AnalyzerConfig::default(),
            generalization_patterns: Self::init_generalization_patterns(),
        }
    }

    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self {
            config,
            generalization_patterns: Self::init_generalization_patterns(),
        }
    }

    fn init_generalization_patterns() -> Vec<(Regex, String)> {
        vec![
            (Regex::new(r"^/.*").unwrap(), "<PATH>".to_string()),
            (Regex::new(r"^[A-Za-z]:\\.*").unwrap(), "<PATH>".to_string()),
            (Regex::new(r"https?://[^\s]+").unwrap(), "<URL>".to_string()),
            (Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(), "<EMAIL>".to_string()),
            (Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap(), "<UUID>".to_string()),
            (Regex::new(r"\b\d+\b").unwrap(), "<NUMBER>".to_string()),
        ]
    }

    pub fn extract(
        &self,
        task_id: &str,
        task_input: &str,
        tool_calls: &[ToolCall],
    ) -> TaskPattern {
        let task_category = self.categorize(task_input);
        let tool_sequence = self.extract_tool_sequence(tool_calls);
        let param_patterns = self.extract_param_patterns(task_input, tool_calls);
        let steps = self.extract_steps(tool_calls);
        let success_indicators = self.extract_success_indicators(tool_calls);
        let reusability_score = self.score_reusability(&task_category, &param_patterns, &steps);

        TaskPattern {
            id: uuid::Uuid::new_v4().to_string(),
            task_category,
            tool_sequence,
            param_patterns,
            success_indicators,
            steps,
            reusability_score,
            source_task_id: task_id.to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn from_task_record(&self, record: &TaskRecord) -> TaskPattern {
        self.extract(
            &record.task_id,
            &record.input,
            &record.tool_calls,
        )
    }

    pub fn extract_and_generalize(
        &self,
        task_id: &str,
        task_input: &str,
        tool_calls: &[ToolCall],
    ) -> TaskPattern {
        let mut pattern = self.extract(task_id, task_input, tool_calls);
        pattern.param_patterns = self.generalize_params(&pattern.param_patterns);
        pattern.reusability_score = self.score_reusability(
            &pattern.task_category,
            &pattern.param_patterns,
            &pattern.steps,
        );
        pattern
    }

    fn generalize_params(&self, params: &[ParamPattern]) -> Vec<ParamPattern> {
        params
            .iter()
            .map(|p| {
                let generalized_examples: Vec<String> = p
                    .examples
                    .iter()
                    .map(|e| self.generalize_value(e))
                    .collect();

                let is_generic = p.is_generic
                    || generalized_examples.iter().any(|e| e.starts_with('<'));

                ParamPattern {
                    name: p.name.clone(),
                    param_type: p.param_type.clone(),
                    is_generic,
                    examples: generalized_examples,
                }
            })
            .collect()
    }

    fn generalize_value(&self, value: &str) -> String {
        let mut result = value.to_string();
        
        if result.starts_with('/') || (result.len() > 1 && result.chars().nth(1) == Some(':')) {
            return "<PATH>".to_string();
        }
        
        for (pattern, replacement) in &self.generalization_patterns {
            if pattern.is_match(&result) {
                result = pattern.replace_all(&result, replacement.as_str()).to_string();
            }
        }
        result
    }

    pub fn find_matching_patterns(
        &self,
        new_pattern: &TaskPattern,
        existing: &[TaskPattern],
    ) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for existing_pattern in existing {
            let similarity = self.calculate_similarity(new_pattern, existing_pattern);
            
            if similarity > 0.3 {
                let match_type = if similarity >= 0.8 {
                    MatchType::Exact
                } else if similarity >= 0.5 {
                    MatchType::Partial
                } else {
                    MatchType::Potential
                };

                let differences = self.find_differences(new_pattern, existing_pattern);

                matches.push(PatternMatch {
                    pattern_id: existing_pattern.id.clone(),
                    similarity,
                    match_type,
                    differences,
                });
            }
        }

        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        matches
    }

    fn calculate_similarity(&self, a: &TaskPattern, b: &TaskPattern) -> f64 {
        let tool_similarity = self.tool_sequence_similarity(&a.tool_sequence, &b.tool_sequence);
        let param_similarity = self.param_similarity(&a.param_patterns, &b.param_patterns);
        let category_similarity = if a.task_category == b.task_category { 1.0 } else { 0.0 };

        tool_similarity * 0.5 + param_similarity * 0.3 + category_similarity * 0.2
    }

    fn tool_sequence_similarity(&self, a: &[ToolCallPattern], b: &[ToolCallPattern]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let lcs_length = self.lcs_length(
            &a.iter().map(|p| p.tool_name.clone()).collect::<Vec<_>>(),
            &b.iter().map(|p| p.tool_name.clone()).collect::<Vec<_>>(),
        );

        let max_len = a.len().max(b.len());
        lcs_length as f64 / max_len as f64
    }

    fn lcs_length(&self, a: &[String], b: &[String]) -> usize {
        let m = a.len();
        let n = b.len();
        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    fn param_similarity(&self, a: &[ParamPattern], b: &[ParamPattern]) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_types: Vec<_> = a.iter().map(|p| &p.param_type).collect();
        let b_types: Vec<_> = b.iter().map(|p| &p.param_type).collect();

        let matches = a_types
            .iter()
            .filter(|t| b_types.contains(t))
            .count();

        let max_len = a_types.len().max(b_types.len());
        matches as f64 / max_len as f64
    }

    fn find_differences(&self, a: &TaskPattern, b: &TaskPattern) -> Vec<String> {
        let mut differences = Vec::new();

        if a.task_category != b.task_category {
            differences.push(format!(
                "Category: '{}' vs '{}'",
                a.task_category, b.task_category
            ));
        }

        let a_tools: Vec<_> = a.tool_sequence.iter().map(|p| &p.tool_name).collect();
        let b_tools: Vec<_> = b.tool_sequence.iter().map(|p| &p.tool_name).collect();

        for tool in a_tools.iter() {
            if !b_tools.contains(tool) {
                differences.push(format!("Extra tool: {}", tool));
            }
        }

        for tool in b_tools.iter() {
            if !a_tools.contains(tool) {
                differences.push(format!("Missing tool: {}", tool));
            }
        }

        differences
    }

    pub fn validate(&self, pattern: &TaskPattern) -> ValidationReport {
        let mut issues = Vec::new();
        let mut score: f64 = 1.0;

        if pattern.tool_sequence.len() < 2 {
            issues.push(ValidationIssue {
                rule: "min_tool_count".to_string(),
                severity: IssueSeverity::Error,
                message: "Pattern must have at least 2 tool calls".to_string(),
            });
            score -= 0.4;
        }

        let success_ratio = if !pattern.steps.is_empty() {
            pattern.steps.iter().filter(|s| s.success).count() as f64 / pattern.steps.len() as f64
        } else {
            0.0
        };

        if success_ratio < 0.8 {
            issues.push(ValidationIssue {
                rule: "success_rate".to_string(),
                severity: IssueSeverity::Warning,
                message: format!("Success rate {:.0}% is below 80%", success_ratio * 100.0),
            });
            score -= 0.2;
        }

        let generic_ratio = if !pattern.param_patterns.is_empty() {
            pattern.param_patterns.iter().filter(|p| p.is_generic).count() as f64
                / pattern.param_patterns.len() as f64
        } else {
            0.0
        };

        if generic_ratio < 0.3 {
            issues.push(ValidationIssue {
                rule: "generic_params".to_string(),
                severity: IssueSeverity::Warning,
                message: "Too few generic parameters for reusability".to_string(),
            });
            score -= 0.1;
        }

        if pattern.reusability_score < self.config.min_reusability_threshold {
            issues.push(ValidationIssue {
                rule: "reusability_score".to_string(),
                severity: IssueSeverity::Warning,
                message: format!(
                    "Reusability score {:.2} below threshold {:.2}",
                    pattern.reusability_score, self.config.min_reusability_threshold
                ),
            });
            score -= 0.3;
        }

        ValidationReport {
            is_valid: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            score: score.max(0.0_f64).min(1.0_f64),
            issues,
        }
    }

    pub fn categorize(&self, input: &str) -> String {
        let input_lower = input.to_lowercase();
        
        if input_lower.contains("search") || input_lower.contains("find") {
            "search".to_string()
        } else if input_lower.contains("code") || input_lower.contains("program") || input_lower.contains("function") {
            "code_generation".to_string()
        } else if input_lower.contains("api") || input_lower.contains("http") || input_lower.contains("fetch") {
            "api_call".to_string()
        } else if input_lower.contains("file") || input_lower.contains("read") || input_lower.contains("write") {
            "file_operation".to_string()
        } else if input_lower.contains("analyze") || input_lower.contains("data") || input_lower.contains("stat") {
            "data_analysis".to_string()
        } else if input_lower.contains("create") || input_lower.contains("draft") {
            "content_creation".to_string()
        } else if input_lower.contains("debug") || input_lower.contains("error") || input_lower.contains("fix") {
            "debugging".to_string()
        } else if input_lower.contains("test") || input_lower.contains("check") {
            "testing".to_string()
        } else {
            "general".to_string()
        }
    }

    fn extract_tool_sequence(&self, tool_calls: &[ToolCall]) -> Vec<ToolCallPattern> {
        tool_calls
            .iter()
            .take(self.config.max_tool_sequence_length)
            .map(|call| {
                let param_schema = self.infer_param_schema(&call.arguments);
                let result_schema = call
                    .result
                    .as_ref()
                    .map(|r| self.infer_param_schema(r))
                    .unwrap_or_default();

                ToolCallPattern {
                    tool_name: call.name.clone(),
                    param_schema,
                    result_schema,
                }
            })
            .collect()
    }

    fn infer_param_schema(&self, value: &serde_json::Value) -> HashMap<String, ParamType> {
        let mut schema = HashMap::new();
        
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                schema.insert(key.clone(), ParamType::from_json_value(val));
            }
        }
        
        schema
    }

    fn extract_param_patterns(&self, _input: &str, tool_calls: &[ToolCall]) -> Vec<ParamPattern> {
        let mut patterns = Vec::<ParamPattern>::new();
        
        for call in tool_calls {
            if let serde_json::Value::Object(args) = &call.arguments {
                for (name, value) in args {
                    let param_type = ParamType::from_json_value(value);
                    let example = value.to_string();
                    let generalized = self.generalize_value(&example);
                    let is_generic = self.is_generic_param(name, value) || generalized.starts_with('<');
                    
                    let existing_idx = patterns.iter().position(|p| p.name == *name);
                    
                    if let Some(idx) = existing_idx {
                        if !patterns[idx].examples.contains(&example) {
                            patterns[idx].examples.push(example);
                        }
                        if generalized.starts_with('<') {
                            patterns[idx].is_generic = true;
                        }
                    } else {
                        patterns.push(ParamPattern {
                            name: name.clone(),
                            param_type: param_type.clone(),
                            is_generic,
                            examples: vec![example],
                        });
                    }
                }
            }
        }
        
        patterns
    }

    fn is_generic_param(&self, name: &str, value: &serde_json::Value) -> bool {
        let generic_names = ["id", "value", "data", "input"];
        let name_lower = name.to_lowercase();
        
        if generic_names.iter().any(|n| name_lower == *n) {
            return true;
        }
        
        if let serde_json::Value::String(s) = value {
            if s.is_empty() || s.starts_with('<') || s.starts_with('{') {
                return true;
            }
            if s.starts_with('/') || (s.len() > 1 && s.chars().nth(1) == Some(':')) {
                return true;
            }
        }
        
        false
    }

    fn extract_steps(&self, tool_calls: &[ToolCall]) -> Vec<ExecutionStep> {
        tool_calls
            .iter()
            .enumerate()
            .map(|(idx, call)| {
                let input_summary = self.summarize_value(&call.arguments);
                let output_summary = call
                    .result
                    .as_ref()
                    .map(|r| self.summarize_value(r))
                    .unwrap_or_else(|| "no output".to_string());
                
                let success = call.result.is_some();

                ExecutionStep {
                    step_number: idx as u32 + 1,
                    tool_name: call.name.clone(),
                    input_summary,
                    output_summary,
                    success,
                }
            })
            .collect()
    }

    fn summarize_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                if s.len() > 50 {
                    format!("{}... ({} chars)", &s[..50], s.len())
                } else {
                    s.clone()
                }
            }
            serde_json::Value::Object(map) => {
                let keys: Vec<&str> = map.keys().take(3).map(|k| k.as_str()).collect();
                format!("object with keys: {}", keys.join(", "))
            }
            serde_json::Value::Array(arr) => {
                format!("array with {} items", arr.len())
            }
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
        }
    }

    fn extract_success_indicators(&self, tool_calls: &[ToolCall]) -> Vec<String> {
        tool_calls
            .iter()
            .filter_map(|call| {
                if call.result.is_some() {
                    Some(format!("{}_success", call.name))
                } else {
                    None
                }
            })
            .collect()
    }

    fn score_reusability(
        &self,
        category: &str,
        param_patterns: &[ParamPattern],
        steps: &[ExecutionStep],
    ) -> f64 {
        let mut score = 0.0;
        
        let category_weight = match category {
            "file_operation" => 0.8,
            "api_call" => 0.9,
            "search" => 0.7,
            "code_generation" => 0.6,
            _ => 0.5,
        };
        score += category_weight * 0.4;
        
        let generic_ratio = if !param_patterns.is_empty() {
            param_patterns.iter().filter(|p| p.is_generic).count() as f64 / param_patterns.len() as f64
        } else {
            0.0
        };
        score += generic_ratio * 0.3;
        
        let success_ratio = if !steps.is_empty() {
            steps.iter().filter(|s| s.success).count() as f64 / steps.len() as f64
        } else {
            0.0
        };
        score += success_ratio * 0.3;
        
        score.max(0.0_f64).min(1.0_f64)
    }

    pub fn is_repeatable(&self, pattern: &TaskPattern) -> bool {
        pattern.reusability_score >= self.config.min_reusability_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_file_operation() {
        let analyzer = PatternAnalyzer::new();
        
        let category = analyzer.categorize("Please read the file /path/to/file.txt");
        assert_eq!(category, "file_operation");
        
        let category = analyzer.categorize("Write some content to a new file");
        assert_eq!(category, "file_operation");
    }

    #[test]
    fn test_categorize_search() {
        let analyzer = PatternAnalyzer::new();
        
        let category = analyzer.categorize("Search for information about Rust");
        assert_eq!(category, "search");
        
        let category = analyzer.categorize("Find all files matching pattern");
        assert_eq!(category, "search");
    }

    #[test]
    fn test_categorize_api_call() {
        let analyzer = PatternAnalyzer::new();
        
        let category = analyzer.categorize("Call the API endpoint");
        assert_eq!(category, "api_call");
        
        let category = analyzer.categorize("Fetch data from HTTP server");
        assert_eq!(category, "api_call");
    }

    #[test]
    fn test_categorize_code_generation() {
        let analyzer = PatternAnalyzer::new();
        
        let category = analyzer.categorize("Write a Python function");
        assert_eq!(category, "code_generation");
        
        let category = analyzer.categorize("Create a Rust program");
        assert_eq!(category, "code_generation");
    }

    #[test]
    fn test_categorize_default() {
        let analyzer = PatternAnalyzer::new();
        
        let category = analyzer.categorize("Hello, how are you?");
        assert_eq!(category, "general");
    }

    #[test]
    fn test_extract_tool_sequence() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "rust"}),
                result: Some(serde_json::json!(["result1", "result2"])),
                duration_ms: 100,
            },
            ToolCall {
                name: "fetch".to_string(),
                arguments: serde_json::json!({"url": "https://example.com"}),
                result: Some(serde_json::json!({"status": 200})),
                duration_ms: 200,
            },
        ];
        
        let sequence = analyzer.extract_tool_sequence(&tool_calls);
        
        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].tool_name, "search");
        assert_eq!(sequence[1].tool_name, "fetch");
    }

    #[test]
    fn test_extract_param_patterns() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "test query", "limit": 10}),
                result: None,
                duration_ms: 100,
            },
        ];
        
        let patterns = analyzer.extract_param_patterns("search for something", &tool_calls);
        
        assert_eq!(patterns.len(), 2);
        
        let query_pattern = patterns.iter().find(|p| p.name == "query").unwrap();
        assert_eq!(query_pattern.param_type, ParamType::String);
        assert!(!query_pattern.is_generic);
        
        let limit_pattern = patterns.iter().find(|p| p.name == "limit").unwrap();
        assert_eq!(limit_pattern.param_type, ParamType::Number);
    }

    #[test]
    fn test_is_generic_param() {
        let analyzer = PatternAnalyzer::new();
        
        assert!(analyzer.is_generic_param("id", &serde_json::json!("user123")));
        assert!(analyzer.is_generic_param("value", &serde_json::json!("test")));
        assert!(analyzer.is_generic_param("data", &serde_json::json!("")));
        
        assert!(!analyzer.is_generic_param("email", &serde_json::json!("user@example.com")));
        assert!(!analyzer.is_generic_param("password", &serde_json::json!("secret")));
        assert!(!analyzer.is_generic_param("query", &serde_json::json!("search term")));
    }

    #[test]
    fn test_extract() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "rust programming"}),
                result: Some(serde_json::json!(["result1"])),
                duration_ms: 100,
            },
            ToolCall {
                name: "fetch".to_string(),
                arguments: serde_json::json!({"url": "https://rust-lang.org"}),
                result: Some(serde_json::json!({"html": "<html>"})),
                duration_ms: 200,
            },
        ];
        
        let pattern = analyzer.extract("task-123", "Search for rust information and fetch details", &tool_calls);
        
        assert_eq!(pattern.source_task_id, "task-123");
        assert_eq!(pattern.task_category, "search");
        assert_eq!(pattern.tool_sequence.len(), 2);
        assert_eq!(pattern.steps.len(), 2);
        assert!(pattern.reusability_score > 0.0);
    }

    #[test]
    fn test_is_repeatable() {
        let analyzer = PatternAnalyzer::new();
        
        let high_score_pattern = TaskPattern {
            id: "1".to_string(),
            task_category: "api_call".to_string(),
            tool_sequence: vec![],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.8,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        };
        
        let low_score_pattern = TaskPattern {
            id: "2".to_string(),
            task_category: "general".to_string(),
            tool_sequence: vec![],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.3,
            source_task_id: "task-2".to_string(),
            created_at: Utc::now(),
        };
        
        assert!(analyzer.is_repeatable(&high_score_pattern));
        assert!(!analyzer.is_repeatable(&low_score_pattern));
    }

    #[test]
    fn test_score_reusability() {
        let analyzer = PatternAnalyzer::new();
        
        let param_patterns = vec![
            ParamPattern {
                name: "query".to_string(),
                param_type: ParamType::String,
                is_generic: true,
                examples: vec![],
            },
        ];
        let steps = vec![
            ExecutionStep {
                step_number: 1,
                tool_name: "api_call".to_string(),
                input_summary: "test".to_string(),
                output_summary: "result".to_string(),
                success: true,
            },
        ];
        
        let score = analyzer.score_reusability("api_call", &param_patterns, &steps);
        assert!(score > 0.6);
        
        let score = analyzer.score_reusability("general", &[], &[]);
        assert!(score < 0.6);
    }

    #[test]
    fn test_summarize_value() {
        let analyzer = PatternAnalyzer::new();
        
        let short_string = serde_json::json!("hello");
        assert_eq!(analyzer.summarize_value(&short_string), "hello");
        
        let long_string = serde_json::json!("this is a very long string that exceeds fifty characters");
        let summary = analyzer.summarize_value(&long_string);
        assert!(summary.contains("..."));
        
        let obj = serde_json::json!({"key1": "value1", "key2": "value2"});
        let summary = analyzer.summarize_value(&obj);
        assert!(summary.contains("key1"));
        
        let arr = serde_json::json!([1, 2, 3, 4, 5]);
        let summary = analyzer.summarize_value(&arr);
        assert!(summary.contains("5 items"));
    }

    #[test]
    fn test_param_type_from_json_value() {
        assert_eq!(ParamType::from_json_value(&serde_json::json!("test")), ParamType::String);
        assert_eq!(ParamType::from_json_value(&serde_json::json!(123)), ParamType::Number);
        assert_eq!(ParamType::from_json_value(&serde_json::json!(true)), ParamType::Boolean);
        assert_eq!(ParamType::from_json_value(&serde_json::json!({})), ParamType::Object);
        assert_eq!(ParamType::from_json_value(&serde_json::json!([])), ParamType::Array);
    }

    #[test]
    fn test_extract_steps() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "test"}),
                result: Some(serde_json::json!(["result"])),
                duration_ms: 100,
            },
            ToolCall {
                name: "fetch".to_string(),
                arguments: serde_json::json!({"url": "test.com"}),
                result: None,
                duration_ms: 50,
            },
        ];
        
        let steps = analyzer.extract_steps(&tool_calls);
        
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].step_number, 1);
        assert_eq!(steps[0].tool_name, "search");
        assert!(steps[0].success);
        
        assert_eq!(steps[1].step_number, 2);
        assert_eq!(steps[1].tool_name, "fetch");
        assert!(!steps[1].success);
    }

    #[test]
    fn test_extract_success_indicators() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({}),
                result: Some(serde_json::json!([])),
                duration_ms: 100,
            },
            ToolCall {
                name: "fetch".to_string(),
                arguments: serde_json::json!({}),
                result: None,
                duration_ms: 100,
            },
        ];
        
        let indicators = analyzer.extract_success_indicators(&tool_calls);
        
        assert_eq!(indicators.len(), 1);
        assert_eq!(indicators[0], "search_success");
    }

    #[test]
    fn test_with_config() {
        let config = AnalyzerConfig {
            min_reusability_threshold: 0.7,
            max_tool_sequence_length: 10,
            enable_deep_analysis: false,
        };
        
        let analyzer = PatternAnalyzer::with_config(config.clone());
        
        let tool_calls = vec![ToolCall {
            name: "test".to_string(),
            arguments: serde_json::json!({}),
            result: None,
            duration_ms: 0,
        }];
        
        let pattern = analyzer.extract("task-1", "test task", &tool_calls);
        assert!(pattern.reusability_score < 0.7 || pattern.reusability_score >= 0.0);
    }

    #[test]
    fn test_from_task_record() {
        let analyzer = PatternAnalyzer::new();
        
        let record = TaskRecord {
            task_id: "task-123".to_string(),
            task_type: "file_operation".to_string(),
            input: "Read file /home/user/data.txt".to_string(),
            tool_calls: vec![
                ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": "/home/user/data.txt"}),
                    result: Some(serde_json::json!("file content")),
                    duration_ms: 50,
                },
            ],
            success: true,
            duration_ms: 100,
        };
        
        let pattern = analyzer.from_task_record(&record);
        
        assert_eq!(pattern.source_task_id, "task-123");
        assert_eq!(pattern.task_category, "file_operation");
    }

    #[test]
    fn test_generalize_value_path() {
        let analyzer = PatternAnalyzer::new();
        
        assert_eq!(analyzer.generalize_value("/home/user/document.pdf"), "<PATH>");
        assert_eq!(analyzer.generalize_value("C:\\Users\\admin\\file.txt"), "<PATH>");
        
        let params = vec![ParamPattern {
            name: "path".to_string(),
            param_type: ParamType::String,
            is_generic: false,
            examples: vec!["/home/user/data.json".to_string()],
        }];
        
        let generalized = analyzer.generalize_params(&params);
        assert!(generalized[0].is_generic, "is_generic should be true after generalization, got: {:?}", generalized);
    }

    #[test]
    fn test_generalize_value_url() {
        let analyzer = PatternAnalyzer::new();
        
        assert_eq!(
            analyzer.generalize_value("https://api.example.com/v1/users"),
            "<URL>"
        );
        assert_eq!(
            analyzer.generalize_value("http://localhost:8080/api"),
            "<URL>"
        );
    }

    #[test]
    fn test_generalize_value_email() {
        let analyzer = PatternAnalyzer::new();
        
        assert_eq!(
            analyzer.generalize_value("user@example.com"),
            "<EMAIL>"
        );
    }

    #[test]
    fn test_generalize_value_uuid() {
        let analyzer = PatternAnalyzer::new();
        
        assert_eq!(
            analyzer.generalize_value("550e8400-e29b-41d4-a716-446655440000"),
            "<UUID>"
        );
    }

    #[test]
    fn test_generalize_value_number() {
        let analyzer = PatternAnalyzer::new();
        
        assert_eq!(analyzer.generalize_value("page=42"), "page=<NUMBER>");
    }

    #[test]
    fn test_extract_and_generalize() {
        let analyzer = PatternAnalyzer::new();
        
        let tool_calls = vec![
            ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "/home/user/data.json"}),
                result: Some(serde_json::json!("content")),
                duration_ms: 50,
            },
        ];
        
        let pattern = analyzer.extract_and_generalize("task-1", "Read file", &tool_calls);
        
        assert_eq!(pattern.param_patterns.len(), 1);
        assert!(pattern.param_patterns[0].is_generic);
    }

    #[test]
    fn test_lcs_length() {
        let analyzer = PatternAnalyzer::new();
        
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(analyzer.lcs_length(&a, &b), 3);
        
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["a".to_string(), "c".to_string(), "e".to_string()];
        assert_eq!(analyzer.lcs_length(&a, &b), 2);
        
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["c".to_string(), "d".to_string()];
        assert_eq!(analyzer.lcs_length(&a, &b), 0);
    }

    #[test]
    fn test_tool_sequence_similarity() {
        let analyzer = PatternAnalyzer::new();
        
        let a = vec![
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
        ];
        let b = vec![
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
        ];
        
        let similarity = analyzer.tool_sequence_similarity(&a, &b);
        assert_eq!(similarity, 1.0);
        
        let b_partial = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: HashMap::new(),
                result_schema: HashMap::new(),
            },
        ];
        
        let similarity = analyzer.tool_sequence_similarity(&a, &b_partial);
        assert!(similarity > 0.3 && similarity < 1.0);
    }

    #[test]
    fn test_find_matching_patterns() {
        let analyzer = PatternAnalyzer::new();
        
        let new_pattern = TaskPattern {
            id: "new-1".to_string(),
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
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.7,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        };
        
        let existing = vec![
            TaskPattern {
                id: "existing-1".to_string(),
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
                param_patterns: vec![],
                success_indicators: vec![],
                steps: vec![],
                reusability_score: 0.8,
                source_task_id: "task-2".to_string(),
                created_at: Utc::now(),
            },
        ];
        
        let matches = analyzer.find_matching_patterns(&new_pattern, &existing);
        
        assert_eq!(matches.len(), 1);
        assert!(matches[0].similarity > 0.7);
        assert_eq!(matches[0].match_type, MatchType::Exact);
    }

    #[test]
    fn test_validate_pattern_valid() {
        let analyzer = PatternAnalyzer::new();
        
        let pattern = TaskPattern {
            id: "1".to_string(),
            task_category: "api_call".to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "fetch".to_string(),
                    param_schema: HashMap::new(),
                    result_schema: HashMap::new(),
                },
                ToolCallPattern {
                    tool_name: "parse".to_string(),
                    param_schema: HashMap::new(),
                    result_schema: HashMap::new(),
                },
            ],
            param_patterns: vec![
                ParamPattern {
                    name: "query".to_string(),
                    param_type: ParamType::String,
                    is_generic: true,
                    examples: vec![],
                },
            ],
            success_indicators: vec!["fetch_success".to_string()],
            steps: vec![
                ExecutionStep {
                    step_number: 1,
                    tool_name: "fetch".to_string(),
                    input_summary: "test".to_string(),
                    output_summary: "result".to_string(),
                    success: true,
                },
            ],
            reusability_score: 0.8,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        };
        
        let report = analyzer.validate(&pattern);
        
        assert!(report.is_valid);
        assert!(report.score > 0.5);
    }

    #[test]
    fn test_validate_pattern_invalid_low_tool_count() {
        let analyzer = PatternAnalyzer::new();
        
        let pattern = TaskPattern {
            id: "1".to_string(),
            task_category: "general".to_string(),
            tool_sequence: vec![],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.3,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        };
        
        let report = analyzer.validate(&pattern);
        
        assert!(!report.is_valid);
        assert!(report.issues.iter().any(|i| i.rule == "min_tool_count"));
    }

    #[test]
    fn test_validate_pattern_low_reusability() {
        let analyzer = PatternAnalyzer::new();
        
        let pattern = TaskPattern {
            id: "1".to_string(),
            task_category: "general".to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "test".to_string(),
                    param_schema: HashMap::new(),
                    result_schema: HashMap::new(),
                },
            ],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![
                ExecutionStep {
                    step_number: 1,
                    tool_name: "test".to_string(),
                    input_summary: "test".to_string(),
                    output_summary: "result".to_string(),
                    success: true,
                },
            ],
            reusability_score: 0.3,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        };
        
        let report = analyzer.validate(&pattern);
        
        assert!(report.issues.iter().any(|i| i.rule == "reusability_score"));
    }
}
