// Chat Service Helper Functions
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use crate::domain::entities::{ChatContextType, MessageRole};
use crate::infrastructure::agents::claude::agent_names::{
    AGENT_CHAT_PROJECT, AGENT_CHAT_TASK, AGENT_IDEATION_TEAM_LEAD, AGENT_MERGER,
    AGENT_ORCHESTRATOR_IDEATION, AGENT_ORCHESTRATOR_IDEATION_READONLY, AGENT_REVIEWER,
    AGENT_REVIEW_CHAT, AGENT_REVIEW_HISTORY, AGENT_WORKER, AGENT_WORKER_TEAM,
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
/// 2. Create the agent definition in `ralphx-plugin/agents/`
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
mod tests {
    use super::*;

    #[test]
    fn test_resolve_agent_review_approved_returns_review_history() {
        let agent = resolve_agent(&ChatContextType::Review, Some("approved"));
        assert_eq!(agent, AGENT_REVIEW_HISTORY);
    }

    #[test]
    fn test_resolve_agent_review_passed_returns_review_chat() {
        let agent = resolve_agent(&ChatContextType::Review, Some("review_passed"));
        assert_eq!(agent, AGENT_REVIEW_CHAT);
    }

    #[test]
    fn test_resolve_agent_review_default_returns_reviewer() {
        let agent = resolve_agent(&ChatContextType::Review, None);
        assert_eq!(agent, AGENT_REVIEWER);
    }

    #[test]
    fn test_resolve_agent_review_other_status_returns_reviewer() {
        let agent = resolve_agent(&ChatContextType::Review, Some("reviewing"));
        assert_eq!(agent, AGENT_REVIEWER);
    }

    #[test]
    fn test_resolve_agent_ideation_accepted_returns_readonly() {
        let agent = resolve_agent(&ChatContextType::Ideation, Some("accepted"));
        assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION_READONLY);
    }

    #[test]
    fn test_resolve_agent_team_mode_ideation_returns_team_lead() {
        let agent =
            resolve_agent_with_team_mode(&ChatContextType::Ideation, None, true);
        assert_eq!(agent, AGENT_IDEATION_TEAM_LEAD);
    }

    #[test]
    fn test_resolve_agent_team_mode_execution_returns_worker_team() {
        let agent =
            resolve_agent_with_team_mode(&ChatContextType::TaskExecution, None, true);
        assert_eq!(agent, AGENT_WORKER_TEAM);
    }

    #[test]
    fn test_resolve_agent_team_mode_project_falls_back_to_default() {
        // Contexts without team variants fall back to defaults
        let agent =
            resolve_agent_with_team_mode(&ChatContextType::Project, None, true);
        assert_eq!(agent, AGENT_CHAT_PROJECT);
    }

    #[test]
    fn test_resolve_agent_team_mode_false_returns_default() {
        let agent =
            resolve_agent_with_team_mode(&ChatContextType::Ideation, None, false);
        assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION);
    }

    #[test]
    fn test_resolve_agent_team_mode_status_overrides_team() {
        // Status-specific rules take priority over team_mode
        let agent =
            resolve_agent_with_team_mode(&ChatContextType::Ideation, Some("accepted"), true);
        assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION_READONLY);
    }

    #[test]
    fn test_context_type_to_process_mapping() {
        assert_eq!(context_type_to_process(&ChatContextType::Ideation), "ideation");
        assert_eq!(context_type_to_process(&ChatContextType::Task), "task");
        assert_eq!(context_type_to_process(&ChatContextType::Project), "project");
        assert_eq!(context_type_to_process(&ChatContextType::TaskExecution), "execution");
        assert_eq!(context_type_to_process(&ChatContextType::Review), "review");
        assert_eq!(context_type_to_process(&ChatContextType::Merge), "merge");
    }
}
