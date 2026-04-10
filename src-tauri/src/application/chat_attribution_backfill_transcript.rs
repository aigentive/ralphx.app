use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::entities::AgentRunUsage;

const HISTORICAL_ATTRIBUTION_SOURCE_PREFIX: &str = "historical_backfill_claude_project_jsonl";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HistoricalClaudeProviderProfile {
    Anthropic,
    ZAi,
    OpenAiCompat,
    Unknown,
    Mixed,
}

impl HistoricalClaudeProviderProfile {
    fn suffix(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::ZAi => "z_ai",
            Self::OpenAiCompat => "openai_compat",
            Self::Unknown => "unknown",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HistoricalTranscriptSummary {
    pub path: PathBuf,
    pub primary_model: Option<String>,
    pub provider_profile: HistoricalClaudeProviderProfile,
    pub total_usage: AgentRunUsage,
    pub assistant_turn_count: usize,
}

pub(crate) type HistoricalTranscriptIndex = HashMap<String, PathBuf>;

impl HistoricalTranscriptSummary {
    pub fn attribution_source(&self) -> String {
        format!(
            "{}_{}",
            HISTORICAL_ATTRIBUTION_SOURCE_PREFIX,
            self.provider_profile.suffix()
        )
    }

    pub fn upstream_provider(&self) -> Option<String> {
        match self.provider_profile {
            HistoricalClaudeProviderProfile::Anthropic => Some("anthropic".to_string()),
            HistoricalClaudeProviderProfile::ZAi => Some("z_ai".to_string()),
            HistoricalClaudeProviderProfile::OpenAiCompat => Some("openai_compat".to_string()),
            HistoricalClaudeProviderProfile::Unknown | HistoricalClaudeProviderProfile::Mixed => {
                None
            }
        }
    }

    pub fn provider_profile_name(&self) -> Option<String> {
        match self.provider_profile {
            HistoricalClaudeProviderProfile::Unknown => None,
            profile => Some(profile.suffix().to_string()),
        }
    }
}

#[derive(Debug, Clone)]
struct AssistantTurnAccumulator {
    message_id: String,
    model: Option<String>,
    usage: AgentRunUsage,
}

impl AssistantTurnAccumulator {
    fn new(message_id: String, model: Option<String>, usage: AgentRunUsage) -> Self {
        Self {
            message_id,
            model,
            usage,
        }
    }

    fn apply(&mut self, model: Option<&str>, usage: &AgentRunUsage) {
        if let Some(model) = model {
            self.model = Some(model.to_string());
        }
        apply_usage_max(&mut self.usage, usage);
    }
}

pub(crate) fn parse_claude_session_transcript_from_path(
    path: &Path,
) -> Result<HistoricalTranscriptSummary, String> {
    let path = path.to_path_buf();

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read transcript {}: {}", path.display(), error))?;

    let mut turns: Vec<AssistantTurnAccumulator> = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("failed to parse transcript {}: {}", path.display(), error))?;

        if value.get("type").and_then(|raw| raw.as_str()) != Some("assistant") {
            continue;
        }

        let message = match value.get("message").and_then(|raw| raw.as_object()) {
            Some(message) => message,
            None => continue,
        };

        let message_id = message
            .get("id")
            .and_then(|raw| raw.as_str())
            .or_else(|| value.get("uuid").and_then(|raw| raw.as_str()))
            .map(str::to_string);

        let Some(message_id) = message_id else {
            continue;
        };

        let model = message.get("model").and_then(|raw| raw.as_str());
        let usage = parse_usage(message.get("usage"));

        match turns.last_mut() {
            Some(current) if current.message_id == message_id => current.apply(model, &usage),
            _ => turns.push(AssistantTurnAccumulator::new(
                message_id,
                model.map(str::to_string),
                usage,
            )),
        }
    }

    if turns.is_empty() {
        return Err(format!(
            "transcript {} had no assistant turns with message ids",
            path.display()
        ));
    }

    let primary_model = turns
        .iter()
        .rev()
        .filter_map(|turn| normalize_model(turn.model.as_deref()).map(str::to_string))
        .next();

    let profile = infer_provider_profile(turns.iter().filter_map(|turn| turn.model.as_deref()));
    let mut total_usage = AgentRunUsage::default();
    for turn in &turns {
        add_usage_u64(&mut total_usage.input_tokens, turn.usage.input_tokens);
        add_usage_u64(&mut total_usage.output_tokens, turn.usage.output_tokens);
        add_usage_u64(
            &mut total_usage.cache_creation_tokens,
            turn.usage.cache_creation_tokens,
        );
        add_usage_u64(
            &mut total_usage.cache_read_tokens,
            turn.usage.cache_read_tokens,
        );
    }

    Ok(HistoricalTranscriptSummary {
        path,
        primary_model,
        provider_profile: profile,
        total_usage,
        assistant_turn_count: turns.len(),
    })
}

fn parse_usage(raw: Option<&serde_json::Value>) -> AgentRunUsage {
    let Some(raw) = raw.and_then(|value| value.as_object()) else {
        return AgentRunUsage::default();
    };

    AgentRunUsage {
        input_tokens: raw.get("input_tokens").and_then(|value| value.as_u64()),
        output_tokens: raw.get("output_tokens").and_then(|value| value.as_u64()),
        cache_creation_tokens: raw
            .get("cache_creation_input_tokens")
            .and_then(|value| value.as_u64()),
        cache_read_tokens: raw
            .get("cache_read_input_tokens")
            .and_then(|value| value.as_u64()),
        estimated_usd: raw.get("cost_usd").and_then(|value| value.as_f64()),
    }
}

fn normalize_model(model: Option<&str>) -> Option<&str> {
    model.filter(|value| !value.is_empty() && *value != "<synthetic>")
}

fn infer_provider_profile<'a>(
    models: impl Iterator<Item = &'a str>,
) -> HistoricalClaudeProviderProfile {
    let profiles: HashSet<HistoricalClaudeProviderProfile> = models
        .filter_map(|model| normalize_model(Some(model)))
        .map(classify_model)
        .collect();

    match profiles.len() {
        0 => HistoricalClaudeProviderProfile::Unknown,
        1 => *profiles.iter().next().unwrap(),
        _ => HistoricalClaudeProviderProfile::Mixed,
    }
}

fn classify_model(model: &str) -> HistoricalClaudeProviderProfile {
    if model.starts_with("glm-") || model.starts_with("z-ai/") {
        HistoricalClaudeProviderProfile::ZAi
    } else if model.starts_with("claude-") || model.starts_with("anthropic/claude") {
        HistoricalClaudeProviderProfile::Anthropic
    } else if model.starts_with("openai/") {
        HistoricalClaudeProviderProfile::OpenAiCompat
    } else {
        HistoricalClaudeProviderProfile::Unknown
    }
}

fn apply_usage_max(target: &mut AgentRunUsage, usage: &AgentRunUsage) {
    max_usage_u64(&mut target.input_tokens, usage.input_tokens);
    max_usage_u64(&mut target.output_tokens, usage.output_tokens);
    max_usage_u64(
        &mut target.cache_creation_tokens,
        usage.cache_creation_tokens,
    );
    max_usage_u64(&mut target.cache_read_tokens, usage.cache_read_tokens);
    if let Some(value) = usage.estimated_usd {
        target.estimated_usd = Some(target.estimated_usd.unwrap_or(0.0).max(value));
    }
}

fn add_usage_u64(total: &mut Option<u64>, value: Option<u64>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0) + value);
    }
}

fn max_usage_u64(total: &mut Option<u64>, value: Option<u64>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0).max(value));
    }
}

pub(crate) fn build_claude_transcript_index(root: &Path) -> HistoricalTranscriptIndex {
    if !root.exists() {
        return HashMap::new();
    }

    let mut index = HashMap::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path.extension().and_then(|raw| raw.to_str()) != Some("jsonl") {
                continue;
            }

            let Some(session_id) = path
                .file_stem()
                .and_then(|raw| raw.to_str())
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            index.entry(session_id.to_string()).or_insert(path);
        }
    }

    index
}
