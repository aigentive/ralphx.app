// Chat Service Helper Functions
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use crate::domain::entities::{ChatContextType, MessageRole};

/// Determines which agent to use based on context type
/// Note: Names must match the `name:` field in ralphx-plugin/agents/*.md
pub fn get_agent_name(context_type: &ChatContextType) -> &'static str {
    match context_type {
        ChatContextType::Ideation => "orchestrator-ideation",
        ChatContextType::Task => "chat-task",
        ChatContextType::Project => "chat-project",
        ChatContextType::TaskExecution => "ralphx-worker",
        ChatContextType::Review => "ralphx-reviewer",
    }
}

/// Get the message role for a context type
pub fn get_assistant_role(context_type: &ChatContextType) -> MessageRole {
    match context_type {
        ChatContextType::TaskExecution => MessageRole::Worker,
        ChatContextType::Review => MessageRole::Reviewer,
        _ => MessageRole::Orchestrator,
    }
}
