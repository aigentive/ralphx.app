// Context-aware routing for chat service
//
// Handles:
// - Working directory resolution based on context type
// - Initial prompt building for different contexts
// - Claude CLI command building

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;

use crate::domain::entities::{
    ChatContextType, ChatConversation, ChatConversationId, ChatMessage, ChatMessageId,
    IdeationSessionId, MessageRole, ProjectId, TaskId,
};
use crate::domain::repositories::{
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::infrastructure::agents::claude::{
    add_prompt_args, build_base_cli_command, configure_spawn, ContentBlockItem, ToolCall,
};

use super::chat_service_helpers::resolve_agent;

/// Resolve the project's working directory from a context
pub async fn resolve_working_directory(
    context_type: ChatContextType,
    context_id: &str,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    default_working_directory: &Path,
) -> PathBuf {
    let project_id = match context_type {
        ChatContextType::Project => Some(ProjectId::from_string(context_id.to_string())),
        ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
            if let Ok(Some(task)) = task_repo
                .get_by_id(&TaskId::from_string(context_id.to_string()))
                .await
            {
                Some(task.project_id)
            } else {
                None
            }
        }
        ChatContextType::Ideation => {
            if let Ok(Some(session)) = ideation_session_repo
                .get_by_id(&IdeationSessionId::from_string(context_id))
                .await
            {
                Some(session.project_id)
            } else {
                None
            }
        }
    };

    if let Some(pid) = project_id {
        if let Ok(Some(project)) = project_repo.get_by_id(&pid).await {
            return PathBuf::from(&project.working_directory);
        }
    }

    default_working_directory.to_path_buf()
}

/// Build the initial prompt for a context
pub fn build_initial_prompt(
    context_type: ChatContextType,
    context_id: &str,
    user_message: &str,
) -> String {
    match context_type {
        ChatContextType::Ideation => {
            format!(
                "RalphX Ideation Session ID: {}\n\nUser's message: {}",
                context_id, user_message
            )
        }
        ChatContextType::Task => {
            format!(
                "RalphX Task ID: {}\n\n\
                 You are helping the user with questions about this specific task.\n\n\
                 User's message: {}",
                context_id, user_message
            )
        }
        ChatContextType::Project => {
            format!(
                "RalphX Project ID: {}\n\n\
                 You are helping the user with project-level questions and suggestions.\n\n\
                 User's message: {}",
                context_id, user_message
            )
        }
        ChatContextType::TaskExecution => {
            format!(
                "RalphX Task Execution ID: {}\n\n{}",
                context_id, user_message
            )
        }
        ChatContextType::Review => {
            format!(
                "RalphX Review Session. Task ID: {}.\n\n\
                 You are reviewing this task. Examine the work, provide feedback, and determine if it meets quality standards.\n\n\
                 User's message: {}",
                context_id, user_message
            )
        }
    }
}

/// Create a Claude CLI command
///
/// `entity_status` is optional and enables dynamic agent resolution based on state.
/// For example, a review context with status "review_passed" will use the review-chat agent.
pub fn build_command(
    cli_path: &Path,
    plugin_dir: &Path,
    conversation: &ChatConversation,
    user_message: &str,
    working_directory: &Path,
    entity_status: Option<&str>,
) -> Command {
    // Compute agent_name using the resolution system (context type + optional status)
    let agent_name = resolve_agent(&conversation.context_type, entity_status);
    eprintln!(
        "[CMD] Setting RALPHX_AGENT_TYPE={} for context {:?} (status: {:?})",
        agent_name, conversation.context_type, entity_status
    );

    // Pass agent_type to build_base_cli_command so it can create dynamic MCP config
    // with the agent type as CLI arg (env vars don't propagate to MCP servers)
    let mut cmd = build_base_cli_command(cli_path, plugin_dir, Some(agent_name));
    cmd.env("RALPHX_AGENT_TYPE", agent_name);

    // Add task scope for task-related contexts
    match conversation.context_type {
        ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
            cmd.env("RALPHX_TASK_ID", &conversation.context_id);
        }
        _ => {}
    }

    // For Review context, ALWAYS start a fresh session. Resuming causes the model
    // to see old "Review already submitted" messages and follow that pattern instead
    // of calling complete_review again. Each review cycle needs a clean slate.
    let should_resume = conversation.claude_session_id.is_some()
        && conversation.context_type != ChatContextType::Review;

    let (prompt, resume_session, agent) = if should_resume {
        let session_id = conversation.claude_session_id.as_ref().unwrap();
        (user_message.to_string(), Some(session_id.as_str()), None)
    } else {
        let initial_prompt = build_initial_prompt(
            conversation.context_type,
            &conversation.context_id,
            user_message,
        );
        (initial_prompt, None, Some(agent_name))
    };

    add_prompt_args(&mut cmd, &prompt, agent, resume_session);
    configure_spawn(&mut cmd, working_directory);

    cmd
}

/// Create a user message based on context type
pub fn create_user_message(
    context_type: ChatContextType,
    context_id: &str,
    content: &str,
    conversation_id: ChatConversationId,
) -> ChatMessage {
    let mut msg = match context_type {
        ChatContextType::Ideation => {
            ChatMessage::user_in_session(IdeationSessionId::from_string(context_id), content)
        }
        ChatContextType::Task | ChatContextType::TaskExecution | ChatContextType::Review => {
            ChatMessage::user_about_task(TaskId::from_string(context_id.to_string()), content)
        }
        ChatContextType::Project => {
            ChatMessage::user_in_project(ProjectId::from_string(context_id.to_string()), content)
        }
    };
    msg.conversation_id = Some(conversation_id);
    msg
}

/// Create an assistant message based on context type
pub fn create_assistant_message(
    context_type: ChatContextType,
    context_id: &str,
    content: &str,
    conversation_id: ChatConversationId,
    tool_calls: &[ToolCall],
    content_blocks: &[ContentBlockItem],
) -> ChatMessage {
    let mut msg = match context_type {
        ChatContextType::Ideation => ChatMessage::orchestrator_in_session(
            IdeationSessionId::from_string(context_id),
            content,
        ),
        ChatContextType::Task => {
            let mut m =
                ChatMessage::user_about_task(TaskId::from_string(context_id.to_string()), content);
            m.role = MessageRole::Orchestrator;
            m
        }
        ChatContextType::Project => {
            let mut m =
                ChatMessage::user_in_project(ProjectId::from_string(context_id.to_string()), content);
            m.role = MessageRole::Orchestrator;
            m
        }
        ChatContextType::TaskExecution => ChatMessage {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: None,
            task_id: Some(TaskId::from_string(context_id.to_string())),
            conversation_id: Some(conversation_id),
            role: MessageRole::Worker,
            content: content.to_string(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: chrono::Utc::now(),
        },
        ChatContextType::Review => ChatMessage {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: None,
            task_id: Some(TaskId::from_string(context_id.to_string())),
            conversation_id: Some(conversation_id),
            role: MessageRole::Reviewer,
            content: content.to_string(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: chrono::Utc::now(),
        },
    };

    msg.conversation_id = Some(conversation_id);

    if !tool_calls.is_empty() {
        msg.tool_calls = Some(serde_json::to_string(tool_calls).unwrap_or_default());
    }
    if !content_blocks.is_empty() {
        msg.content_blocks = Some(serde_json::to_string(content_blocks).unwrap_or_default());
    }

    msg
}
