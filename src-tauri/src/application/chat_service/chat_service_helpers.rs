// Chat Service Helper Functions
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use crate::domain::entities::{ChatContextType, MessageRole};

/// Agent Resolution System
///
/// Determines which agent to use based on context type AND optionally entity status.
/// This enables dynamic agent switching based on workflow state.
///
/// ## Examples
///
/// - Review context + "reviewing" status → `ralphx-reviewer` (AI performs review)
/// - Review context + "review_passed" status → `ralphx-review-chat` (user discusses with AI)
///
/// ## Adding New Rules
///
/// 1. Add a pattern to `resolve_agent()` for status-specific behavior
/// 2. Create the agent definition in `ralphx-plugin/agents/`
/// 3. Add tools to MCP allowlist in `ralphx-mcp-server/src/tools.ts`
///
/// Priority: Status-specific rules are checked first, then defaults.
pub fn resolve_agent(context_type: &ChatContextType, entity_status: Option<&str>) -> &'static str {
    // Status-specific rules (checked first)
    if let Some(status) = entity_status {
        match (context_type, status) {
            // Review: after AI review passes, use chat agent for user discussion
            (ChatContextType::Review, "review_passed") => return "ralphx-review-chat",

            // Review: approved tasks use read-only history agent for retrospective discussion
            (ChatContextType::Review, "approved") => return "ralphx-review-history",

            // Ideation: accepted plans use read-only agent (no mutation tools)
            (ChatContextType::Ideation, "accepted") => return "orchestrator-ideation-readonly",

            // Add more status-specific rules here as needed:
            // (ChatContextType::Task, "blocked") => return "task-unblock-helper",
            // (ChatContextType::TaskExecution, "failed") => return "failure-analyzer",
            _ => {}
        }
    }

    // Default rules (context-only, backward compatible)
    match context_type {
        ChatContextType::Ideation => "orchestrator-ideation",
        ChatContextType::Task => "chat-task",
        ChatContextType::Project => "chat-project",
        ChatContextType::TaskExecution => "ralphx-worker",
        ChatContextType::Review => "ralphx-reviewer",
        ChatContextType::Merge => "ralphx-merger",
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
        assert_eq!(agent, "ralphx-review-history");
    }

    #[test]
    fn test_resolve_agent_review_passed_returns_review_chat() {
        let agent = resolve_agent(&ChatContextType::Review, Some("review_passed"));
        assert_eq!(agent, "ralphx-review-chat");
    }

    #[test]
    fn test_resolve_agent_review_default_returns_reviewer() {
        let agent = resolve_agent(&ChatContextType::Review, None);
        assert_eq!(agent, "ralphx-reviewer");
    }

    #[test]
    fn test_resolve_agent_review_other_status_returns_reviewer() {
        let agent = resolve_agent(&ChatContextType::Review, Some("reviewing"));
        assert_eq!(agent, "ralphx-reviewer");
    }

    #[test]
    fn test_resolve_agent_ideation_accepted_returns_readonly() {
        let agent = resolve_agent(&ChatContextType::Ideation, Some("accepted"));
        assert_eq!(agent, "orchestrator-ideation-readonly");
    }
}
