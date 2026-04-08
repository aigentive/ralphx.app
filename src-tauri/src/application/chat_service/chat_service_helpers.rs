// Chat Service Helper Functions
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use crate::domain::entities::{ChatContextType, MessageRole};
use crate::domain::agents::AgentHarnessKind;
use crate::infrastructure::agents::claude::agent_names::{
    AGENT_CHAT_PROJECT, AGENT_CHAT_TASK, AGENT_IDEATION_TEAM_LEAD, AGENT_MERGER,
    AGENT_ORCHESTRATOR_IDEATION, AGENT_ORCHESTRATOR_IDEATION_READONLY, AGENT_PLAN_VERIFIER,
    AGENT_REVIEWER, AGENT_REVIEW_CHAT, AGENT_REVIEW_HISTORY, AGENT_WORKER, AGENT_WORKER_TEAM,
};

/// Agent Resolution System
///
/// Determines which agent to use based on context type AND optionally entity status.
/// This enables dynamic agent switching based on workflow state.
///
/// When `team_mode` is true, the team-lead agent variant is returned if available.
/// If no team variant exists, falls back to default.
///
/// ## Examples
///
/// - Review context + "reviewing" status → `ralphx-reviewer` (AI performs review)
/// - Review context + "review_passed" status → `ralphx-review-chat` (user discusses with AI)
/// - Ideation context + team_mode=true → team-lead variant (if configured)
///
/// ## Adding New Rules
///
/// 1. Add a pattern to `resolve_agent()` for status-specific behavior
/// 2. Create the agent definition in `plugins/app/agents/`
/// 3. Add tools to MCP allowlist in `ralphx-mcp-server/src/tools.ts`
///
/// Priority: Status-specific rules are checked first, then team_mode, then defaults.
pub fn resolve_agent(context_type: &ChatContextType, entity_status: Option<&str>) -> &'static str {
    resolve_agent_with_team_mode(context_type, entity_status, false)
}

/// Resolve agent with explicit team_mode flag.
///
/// When `team_mode` is true, returns the team-lead variant for the context type's
/// process if one is configured via process mapping YAML. Falls back to the
/// default agent otherwise.
pub fn resolve_agent_with_team_mode(
    context_type: &ChatContextType,
    entity_status: Option<&str>,
    team_mode: bool,
) -> &'static str {
    // Status-specific rules (checked first, even in team mode)
    if let Some(status) = entity_status {
        match (context_type, status) {
            // Review: after AI review passes, use chat agent for user discussion
            (ChatContextType::Review, "review_passed") => return AGENT_REVIEW_CHAT,

            // Review: approved tasks use read-only history agent for retrospective discussion
            (ChatContextType::Review, "approved") => return AGENT_REVIEW_HISTORY,

            // Ideation: verification child sessions route to the dedicated plan-verifier agent
            (ChatContextType::Ideation, "verification") => return AGENT_PLAN_VERIFIER,

            // Ideation: accepted plans use read-only agent (no mutation tools)
            (ChatContextType::Ideation, "accepted") => return AGENT_ORCHESTRATOR_IDEATION_READONLY,

            _ => {}
        }
    }

    // Team mode: resolve to team-lead variant
    if team_mode {
        match context_type {
            ChatContextType::Ideation => return AGENT_IDEATION_TEAM_LEAD,
            ChatContextType::TaskExecution => return AGENT_WORKER_TEAM,
            _ => {
                tracing::debug!(
                    context_type = %context_type,
                    "resolve_agent: team_mode=true, no team variant for context — using default"
                );
            }
        }
    }

    // Default rules (context-only, backward compatible)
    match context_type {
        ChatContextType::Ideation => AGENT_ORCHESTRATOR_IDEATION,
        ChatContextType::Task => AGENT_CHAT_TASK,
        ChatContextType::Project => AGENT_CHAT_PROJECT,
        ChatContextType::TaskExecution => AGENT_WORKER,
        ChatContextType::Review => AGENT_REVIEWER,
        ChatContextType::Merge => AGENT_MERGER,
    }
}

/// Codex phase 1 runs in solo mode even if the session metadata still carries
/// a team-mode value. Claude remains the only harness that honors team mode.
pub fn effective_team_mode_for_harness(
    requested_team_mode: bool,
    harness: AgentHarnessKind,
) -> bool {
    requested_team_mode && harness == AgentHarnessKind::Claude
}

/// Resolve a provider harness while preserving legacy pre-Codex Claude sessions.
///
/// During the additive migration window, a persisted `claude_session_id` without a
/// provider-neutral harness still implies Claude.
pub fn provider_harness_or_default(
    provider_harness: Option<AgentHarnessKind>,
    legacy_claude_session_id: Option<&str>,
    default_harness: AgentHarnessKind,
) -> AgentHarnessKind {
    provider_harness.unwrap_or_else(|| {
        legacy_claude_session_id
            .map(|_| AgentHarnessKind::Claude)
            .unwrap_or(default_harness)
    })
}

/// Map ChatContextType to process name for team config lookup
pub fn context_type_to_process(context_type: &ChatContextType) -> &'static str {
    match context_type {
        ChatContextType::Ideation => "ideation",
        ChatContextType::Task => "task",
        ChatContextType::Project => "project",
        ChatContextType::TaskExecution => "execution",
        ChatContextType::Review => "review",
        ChatContextType::Merge => "merge",
    }
}

/// Legacy function for backward compatibility
/// Use `resolve_agent()` when entity status is available
pub fn get_agent_name(context_type: &ChatContextType) -> &'static str {
    resolve_agent(context_type, None)
}

/// Get the message role for a context type
pub fn get_assistant_role(context_type: &ChatContextType) -> MessageRole {
    match context_type {
        ChatContextType::TaskExecution => MessageRole::Worker,
        ChatContextType::Review => MessageRole::Reviewer,
        ChatContextType::Merge => MessageRole::Merger,
        _ => MessageRole::Orchestrator,
    }
}

#[cfg(test)]
#[path = "chat_service_helpers_tests.rs"]
mod tests;
