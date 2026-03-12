use chrono::Utc;

use super::hand::{
    ExecutionConfig, Guardrail, GuardrailAction, Hand, HandCategory,
    HandState, MetricDefinition, ToolDefinition,
};
use super::schedule::ScheduleType;

pub fn get_preset_hands() -> Vec<Hand> {
    vec![
        researcher_hand(),
        collector_hand(),
        lead_generator_hand(),
        predictor_hand(),
    ]
}

pub fn researcher_hand() -> Hand {
    let now = Utc::now();
    Hand {
        id: "researcher".to_string(),
        name: "Deep Researcher".to_string(),
        description: "Cross-references multiple sources, evaluates credibility using CRAAP criteria, generates cited reports".to_string(),
        category: HandCategory::Research,
        schedule: Some(ScheduleType::Cron("0 6 * * *".to_string())),
        system_prompt: r#"You are a deep autonomous researcher. Your task is to:
1. Research the given topic thoroughly
2. Cross-reference multiple sources
3. Evaluate credibility using CRAAP (Currency, Relevance, Authority, Accuracy, Purpose)
4. Generate well-cited reports in APA format
5. Support multiple languages

Always cite your sources and provide evidence for claims."#.to_string(),
        skill_id: None,
        tools: vec![
            ToolDefinition {
                name: "web_search".to_string(),
                description: "Search the web for information".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "web_fetch".to_string(),
                description: "Fetch content from a URL".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "knowledge_graph_update".to_string(),
                description: "Update the knowledge graph with findings".to_string(),
                required: false,
            },
        ],
        guardrails: vec![],
        metrics: vec![
            MetricDefinition {
                name: "sources_consulted".to_string(),
                description: "Number of sources consulted".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "reports_generated".to_string(),
                description: "Number of research reports generated".to_string(),
                unit: "count".to_string(),
            },
        ],
        enabled: false,
        created_at: now,
        updated_at: now,
        version: "1.0.0".to_string(),
        output_channels: vec![],
        execution_config: ExecutionConfig::default(),
        state: HandState::default(),
        predictive_config: None,
        skill_calls: vec![],
    }
}

pub fn collector_hand() -> Hand {
    let now = Utc::now();
    Hand {
        id: "collector".to_string(),
        name: "Data Collector".to_string(),
        description: "Collects data from various sources on a scheduled basis".to_string(),
        category: HandCategory::Collection,
        schedule: Some(ScheduleType::Cron("0 */4 * * *".to_string())),
        system_prompt: r#"You are an OSINT-grade intelligence collector. Your task is to:
1. Monitor specified targets (companies, people, topics)
2. Continuously collect information
3. Detect changes and shifts
4. Build and update a knowledge graph
5. Send critical alerts when something important changes

Be thorough and accurate in your intelligence gathering."#.to_string(),
        skill_id: None,
        tools: vec![
            ToolDefinition {
                name: "monitor".to_string(),
                description: "Monitor a target for changes".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "change_detection".to_string(),
                description: "Detect changes in monitored data".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "knowledge_graph_update".to_string(),
                description: "Update the knowledge graph".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "alert".to_string(),
                description: "Send an alert notification".to_string(),
                required: true,
            },
        ],
        guardrails: vec![],
        metrics: vec![
            MetricDefinition {
                name: "targets_monitored".to_string(),
                description: "Number of targets being monitored".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "changes_detected".to_string(),
                description: "Number of changes detected".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "alerts_sent".to_string(),
                description: "Number of alerts sent".to_string(),
                unit: "count".to_string(),
            },
        ],
        enabled: false,
        created_at: now,
        updated_at: now,
        version: "1.0.0".to_string(),
        output_channels: vec![],
        execution_config: ExecutionConfig::default(),
        state: HandState::default(),
        predictive_config: None,
        skill_calls: vec![],
    }
}

pub fn lead_generator_hand() -> Hand {
    let now = Utc::now();
    Hand {
        id: "lead".to_string(),
        name: "Lead Generator".to_string(),
        description: "Discovers prospects matching ICP, enriches with web research, scores 0-100, deduplicates, delivers in CSV/JSON/Markdown".to_string(),
        category: HandCategory::Research,
        schedule: Some(ScheduleType::Cron("0 8 * * 1-5".to_string())),
        system_prompt: r#"You are a lead generation agent. Your task is to:
1. Discover prospects matching the Ideal Customer Profile (ICP)
2. Enrich prospects with web research
3. Score leads on a 0-100 scale
4. Deduplicate against existing database
5. Deliver qualified leads in CSV/JSON/Markdown format

Focus on quality leads that match the ICP criteria."#.to_string(),
        skill_id: None,
        tools: vec![
            ToolDefinition {
                name: "discover_prospects".to_string(),
                description: "Discover potential prospects".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "enrich_data".to_string(),
                description: "Enrich prospect data with research".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "score_lead".to_string(),
                description: "Score a lead 0-100".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "deduplicate".to_string(),
                description: "Remove duplicate leads".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "export_csv".to_string(),
                description: "Export leads to CSV".to_string(),
                required: false,
            },
        ],
        guardrails: vec![Guardrail {
            action: GuardrailAction::Log,
            description: "Log all lead generation activities".to_string(),
        }],
        metrics: vec![
            MetricDefinition {
                name: "prospects_discovered".to_string(),
                description: "Number of prospects discovered".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "leads_generated".to_string(),
                description: "Number of qualified leads generated".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "avg_score".to_string(),
                description: "Average lead score".to_string(),
                unit: "score".to_string(),
            },
        ],
        enabled: false,
        created_at: now,
        updated_at: now,
        version: "1.0.0".to_string(),
        output_channels: vec![],
        execution_config: ExecutionConfig::default(),
        state: HandState::default(),
        predictive_config: None,
        skill_calls: vec![],
    }
}

pub fn predictor_hand() -> Hand {
    let now = Utc::now();
    Hand {
        id: "predictor".to_string(),
        name: "Forecasting Engine".to_string(),
        description: "Collects signals from multiple sources, builds calibrated reasoning chains, makes predictions with confidence intervals, tracks accuracy using Brier scores".to_string(),
        category: HandCategory::Prediction,
        schedule: Some(ScheduleType::Cron("0 0 * * *".to_string())),
        system_prompt: r#"You are a superforecasting engine. Your task is to:
1. Collect signals from multiple sources
2. Build calibrated reasoning chains
3. Make predictions with confidence intervals
4. Track your own accuracy using Brier scores
5. Optionally run in 'contrarian mode' that deliberately argues against consensus

Be honest about uncertainty and update your beliefs based on evidence."#.to_string(),
        skill_id: None,
        tools: vec![
            ToolDefinition {
                name: "collect_signals".to_string(),
                description: "Collect signals from various sources".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "calibrate_reasoning".to_string(),
                description: "Build calibrated reasoning chains".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "make_prediction".to_string(),
                description: "Make a prediction with confidence interval".to_string(),
                required: true,
            },
            ToolDefinition {
                name: "track_accuracy".to_string(),
                description: "Track prediction accuracy using Brier score".to_string(),
                required: true,
            },
        ],
        guardrails: vec![],
        metrics: vec![
            MetricDefinition {
                name: "predictions_made".to_string(),
                description: "Number of predictions made".to_string(),
                unit: "count".to_string(),
            },
            MetricDefinition {
                name: "brier_score".to_string(),
                description: "Brier score (lower is better)".to_string(),
                unit: "score".to_string(),
            },
            MetricDefinition {
                name: "accuracy_rate".to_string(),
                description: "Overall accuracy rate".to_string(),
                unit: "percentage".to_string(),
            },
        ],
        enabled: false,
        created_at: now,
        updated_at: now,
        version: "1.0.0".to_string(),
        output_channels: vec![],
        execution_config: ExecutionConfig::default(),
        state: HandState::default(),
        predictive_config: None,
        skill_calls: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_researcher_hand() {
        let hand = researcher_hand();
        assert_eq!(hand.id, "researcher");
        assert_eq!(hand.category, HandCategory::Research);
        assert!(!hand.system_prompt.is_empty());
    }

    #[test]
    fn test_collector_hand() {
        let hand = collector_hand();
        assert_eq!(hand.id, "collector");
        assert_eq!(hand.category, HandCategory::Collection);
    }

    #[test]
    fn test_lead_generator_hand() {
        let hand = lead_generator_hand();
        assert_eq!(hand.id, "lead");
        assert_eq!(hand.category, HandCategory::Research);
        assert!(!hand.guardrails.is_empty());
    }

    #[test]
    fn test_predictor_hand() {
        let hand = predictor_hand();
        assert_eq!(hand.id, "predictor");
        assert_eq!(hand.category, HandCategory::Prediction);
    }

    #[test]
    fn test_get_preset_hands() {
        let hands = get_preset_hands();
        assert_eq!(hands.len(), 4);
        
        let ids: Vec<_> = hands.iter().map(|h| h.id.clone()).collect();
        assert!(ids.contains(&"researcher".to_string()));
        assert!(ids.contains(&"collector".to_string()));
        assert!(ids.contains(&"lead".to_string()));
        assert!(ids.contains(&"predictor".to_string()));
    }
}
