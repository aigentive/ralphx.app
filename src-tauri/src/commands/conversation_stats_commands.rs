use std::collections::BTreeMap;

use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    AgentRun, AgentRunUsage, ChatConversation, ChatMessage, MessageRole,
};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageTotalsResponse {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub estimated_usd: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageBucketResponse {
    pub key: String,
    pub count: u64,
    pub usage: UsageTotalsResponse,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationUsageCoverageResponse {
    pub provider_message_count: u64,
    pub provider_messages_with_usage: u64,
    pub run_count: u64,
    pub runs_with_usage: u64,
    pub effective_totals_source: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAttributionCoverageResponse {
    pub provider_message_count: u64,
    pub provider_messages_with_attribution: u64,
    pub run_count: u64,
    pub runs_with_attribution: u64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationStatsResponse {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub provider_harness: Option<String>,
    pub upstream_provider: Option<String>,
    pub provider_profile: Option<String>,
    pub attribution_backfill_status: Option<String>,
    pub attribution_backfill_source: Option<String>,
    pub message_usage_totals: UsageTotalsResponse,
    pub run_usage_totals: UsageTotalsResponse,
    pub effective_usage_totals: UsageTotalsResponse,
    pub usage_coverage: ConversationUsageCoverageResponse,
    pub attribution_coverage: ConversationAttributionCoverageResponse,
    pub by_harness: Vec<UsageBucketResponse>,
    pub by_upstream_provider: Vec<UsageBucketResponse>,
    pub by_model: Vec<UsageBucketResponse>,
    pub by_effort: Vec<UsageBucketResponse>,
}

pub fn build_conversation_stats_response(
    conversation: &ChatConversation,
    messages: &[ChatMessage],
    runs: &[AgentRun],
) -> ConversationStatsResponse {
    let provider_messages: Vec<&ChatMessage> = messages
        .iter()
        .filter(|message| is_provider_message(message.role))
        .collect();
    let provider_messages_with_usage: Vec<&ChatMessage> = provider_messages
        .iter()
        .copied()
        .filter(message_has_usage)
        .collect();
    let provider_messages_with_attribution = provider_messages
        .iter()
        .copied()
        .filter(message_has_attribution)
        .count() as u64;

    let runs_with_usage: Vec<&AgentRun> = runs.iter().filter(run_has_usage).collect();
    let runs_with_attribution = runs.iter().filter(|run| run_has_attribution(run)).count() as u64;

    let message_usage_totals = sum_message_usage(&provider_messages_with_usage);
    let run_usage_totals = sum_run_usage(&runs_with_usage);
    let effective_usage_source = if !provider_messages_with_usage.is_empty() {
        "messages"
    } else if !runs_with_usage.is_empty() {
        "runs"
    } else {
        "none"
    };

    let (by_harness, by_upstream_provider, by_model, by_effort, effective_usage_totals) =
        match effective_usage_source {
            "messages" => (
                aggregate_message_buckets(
                    &provider_messages_with_usage,
                    |message| message.provider_harness.map(|value| value.to_string()),
                ),
                aggregate_message_buckets(
                    &provider_messages_with_usage,
                    |message| message.upstream_provider.clone(),
                ),
                aggregate_message_buckets(
                    &provider_messages_with_usage,
                    |message| message.effective_model_id.clone(),
                ),
                aggregate_message_buckets(
                    &provider_messages_with_usage,
                    |message| {
                        message
                            .effective_effort
                            .clone()
                            .or_else(|| message.logical_effort.map(|value| value.to_string()))
                    },
                ),
                message_usage_totals.clone(),
            ),
            "runs" => (
                aggregate_run_buckets(&runs_with_usage, |run| run.harness.map(|value| value.to_string())),
                aggregate_run_buckets(&runs_with_usage, |run| run.upstream_provider.clone()),
                aggregate_run_buckets(&runs_with_usage, |run| run.effective_model_id.clone()),
                aggregate_run_buckets(&runs_with_usage, |run| {
                    run.effective_effort
                        .clone()
                        .or_else(|| run.logical_effort.map(|value| value.to_string()))
                }),
                run_usage_totals.clone(),
            ),
            _ => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                UsageTotalsResponse::default(),
            ),
        };

    ConversationStatsResponse {
        conversation_id: conversation.id.as_str(),
        context_type: conversation.context_type.to_string(),
        context_id: conversation.context_id.clone(),
        provider_harness: conversation.provider_harness.map(|value| value.to_string()),
        upstream_provider: conversation.upstream_provider.clone(),
        provider_profile: conversation.provider_profile.clone(),
        attribution_backfill_status: conversation
            .attribution_backfill_status
            .map(|value| value.to_string()),
        attribution_backfill_source: conversation.attribution_backfill_source.clone(),
        message_usage_totals,
        run_usage_totals,
        effective_usage_totals,
        usage_coverage: ConversationUsageCoverageResponse {
            provider_message_count: provider_messages.len() as u64,
            provider_messages_with_usage: provider_messages_with_usage.len() as u64,
            run_count: runs.len() as u64,
            runs_with_usage: runs_with_usage.len() as u64,
            effective_totals_source: effective_usage_source.to_string(),
        },
        attribution_coverage: ConversationAttributionCoverageResponse {
            provider_message_count: provider_messages.len() as u64,
            provider_messages_with_attribution,
            run_count: runs.len() as u64,
            runs_with_attribution,
        },
        by_harness,
        by_upstream_provider,
        by_model,
        by_effort,
    }
}

#[tauri::command]
pub async fn get_agent_conversation_stats(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ConversationStatsResponse>, String> {
    let conversation_id =
        crate::domain::entities::ChatConversationId::from_string(conversation_id);
    let Some(conversation) = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };

    let messages = state
        .chat_message_repo
        .get_by_conversation(&conversation_id)
        .await
        .map_err(|error| error.to_string())?;
    let runs = state
        .agent_run_repo
        .get_by_conversation(&conversation_id)
        .await
        .map_err(|error| error.to_string())?;

    Ok(Some(build_conversation_stats_response(
        &conversation,
        &messages,
        &runs,
    )))
}

#[derive(Default, Clone)]
struct UsageAccumulator {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    estimated_usd: Option<f64>,
}

impl UsageAccumulator {
    fn add_usage(&mut self, usage: &AgentRunUsage) {
        self.input_tokens += usage.input_tokens.unwrap_or(0);
        self.output_tokens += usage.output_tokens.unwrap_or(0);
        self.cache_creation_tokens += usage.cache_creation_tokens.unwrap_or(0);
        self.cache_read_tokens += usage.cache_read_tokens.unwrap_or(0);
        if let Some(value) = usage.estimated_usd {
            self.estimated_usd = Some(self.estimated_usd.unwrap_or(0.0) + value);
        }
    }

    fn to_response(&self) -> UsageTotalsResponse {
        UsageTotalsResponse {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_tokens: self.cache_creation_tokens,
            cache_read_tokens: self.cache_read_tokens,
            estimated_usd: self.estimated_usd,
        }
    }
}

impl Default for UsageTotalsResponse {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            estimated_usd: None,
        }
    }
}

fn is_provider_message(role: MessageRole) -> bool {
    !matches!(role, MessageRole::User | MessageRole::System)
}

fn message_has_usage(message: &&ChatMessage) -> bool {
    message.input_tokens.is_some()
        || message.output_tokens.is_some()
        || message.cache_creation_tokens.is_some()
        || message.cache_read_tokens.is_some()
        || message.estimated_usd.is_some()
}

fn message_has_attribution(message: &&ChatMessage) -> bool {
    message.provider_harness.is_some()
        || message.provider_session_id.is_some()
        || message.upstream_provider.is_some()
        || message.provider_profile.is_some()
        || message.effective_model_id.is_some()
        || message.effective_effort.is_some()
}

fn run_has_usage(run: &&AgentRun) -> bool {
    run.input_tokens.is_some()
        || run.output_tokens.is_some()
        || run.cache_creation_tokens.is_some()
        || run.cache_read_tokens.is_some()
        || run.estimated_usd.is_some()
}

fn run_has_attribution(run: &AgentRun) -> bool {
    run.harness.is_some()
        || run.provider_session_id.is_some()
        || run.upstream_provider.is_some()
        || run.provider_profile.is_some()
        || run.effective_model_id.is_some()
        || run.effective_effort.is_some()
}

fn sum_message_usage(messages: &[&ChatMessage]) -> UsageTotalsResponse {
    let mut total = UsageAccumulator::default();
    for message in messages {
        total.add_usage(&AgentRunUsage {
            input_tokens: message.input_tokens,
            output_tokens: message.output_tokens,
            cache_creation_tokens: message.cache_creation_tokens,
            cache_read_tokens: message.cache_read_tokens,
            estimated_usd: message.estimated_usd,
        });
    }
    total.to_response()
}

fn sum_run_usage(runs: &[&AgentRun]) -> UsageTotalsResponse {
    let mut total = UsageAccumulator::default();
    for run in runs {
        total.add_usage(&AgentRunUsage {
            input_tokens: run.input_tokens,
            output_tokens: run.output_tokens,
            cache_creation_tokens: run.cache_creation_tokens,
            cache_read_tokens: run.cache_read_tokens,
            estimated_usd: run.estimated_usd,
        });
    }
    total.to_response()
}

fn aggregate_message_buckets(
    messages: &[&ChatMessage],
    key_fn: impl Fn(&ChatMessage) -> Option<String>,
) -> Vec<UsageBucketResponse> {
    let mut buckets: BTreeMap<String, (u64, UsageAccumulator)> = BTreeMap::new();
    for message in messages {
        let key = key_fn(message).unwrap_or_else(|| "unknown".to_string());
        let entry = buckets
            .entry(key)
            .or_insert_with(|| (0, UsageAccumulator::default()));
        entry.0 += 1;
        entry.1.add_usage(&AgentRunUsage {
            input_tokens: message.input_tokens,
            output_tokens: message.output_tokens,
            cache_creation_tokens: message.cache_creation_tokens,
            cache_read_tokens: message.cache_read_tokens,
            estimated_usd: message.estimated_usd,
        });
    }
    buckets
        .into_iter()
        .map(|(key, (count, usage))| UsageBucketResponse {
            key,
            count,
            usage: usage.to_response(),
        })
        .collect()
}

fn aggregate_run_buckets(
    runs: &[&AgentRun],
    key_fn: impl Fn(&AgentRun) -> Option<String>,
) -> Vec<UsageBucketResponse> {
    let mut buckets: BTreeMap<String, (u64, UsageAccumulator)> = BTreeMap::new();
    for run in runs {
        let key = key_fn(run).unwrap_or_else(|| "unknown".to_string());
        let entry = buckets
            .entry(key)
            .or_insert_with(|| (0, UsageAccumulator::default()));
        entry.0 += 1;
        entry.1.add_usage(&AgentRunUsage {
            input_tokens: run.input_tokens,
            output_tokens: run.output_tokens,
            cache_creation_tokens: run.cache_creation_tokens,
            cache_read_tokens: run.cache_read_tokens,
            estimated_usd: run.estimated_usd,
        });
    }
    buckets
        .into_iter()
        .map(|(key, (count, usage))| UsageBucketResponse {
            key,
            count,
            usage: usage.to_response(),
        })
        .collect()
}
