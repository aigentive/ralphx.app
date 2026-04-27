use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::{Map as JsonMap, Value as JsonValue};
use tauri::Emitter;

use crate::application::chat_service::create_assistant_message;
use crate::application::{AgentMessageCreatedPayload, AppState};
use crate::domain::entities::{
    AgentConversationWorkspace, ChatContextType, ChatConversationId, ChatMessage,
};
use crate::domain::repositories::external_events_repository::ExternalEventRecord;
use crate::error::AppResult;

const BRIDGE_SOURCE: &str = "project_agent_ideation_bridge";
const MAX_BRIDGE_EVENT_REPLAY: i64 = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWorkspaceBridgeMessage {
    pub event_key: String,
    pub event_type: String,
    pub content: String,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
}

pub async fn reconcile_agent_workspace_bridge_messages(
    state: &AppState,
    conversation_id: &ChatConversationId,
    app: Option<&tauri::AppHandle>,
) -> AppResult<usize> {
    let Some(conversation) = state
        .chat_conversation_repo
        .get_by_id(conversation_id)
        .await?
    else {
        return Ok(0);
    };
    if conversation.context_type != ChatContextType::Project {
        return Ok(0);
    }

    let Some(workspace) = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(conversation_id)
        .await?
    else {
        return Ok(0);
    };

    let existing_messages = state
        .chat_message_repo
        .get_by_conversation(conversation_id)
        .await?;
    let (mut existing_bridge_keys, removed_invalid_count) =
        remove_invalid_bridge_messages(state, conversation_id, &workspace, existing_messages)
            .await?;
    if removed_invalid_count > 0 {
        refresh_conversation_stats(state, conversation_id).await?;
    }

    let Some(session_id) = workspace.linked_ideation_session_id.as_ref() else {
        return Ok(0);
    };
    let events = state
        .external_events_repo
        .get_events_after_cursor(
            &[workspace.project_id.as_str().to_string()],
            0,
            MAX_BRIDGE_EVENT_REPLAY,
        )
        .await?;

    let mut created_count = 0;
    for event in events {
        let Some(bridge_message) = bridge_message_from_external_event(&event, session_id.as_str())
        else {
            continue;
        };
        if !existing_bridge_keys.insert(bridge_message.event_key.clone()) {
            continue;
        }

        let created = create_bridge_chat_message(
            state,
            &conversation,
            conversation_id,
            session_id.as_str(),
            bridge_message,
        )
        .await?;
        created_count += 1;

        if let Some(app) = app {
            let _ = app.emit(
                "agent:message_created",
                AgentMessageCreatedPayload {
                    message_id: created.id.as_str().to_string(),
                    conversation_id: conversation_id.as_str().to_string(),
                    context_type: conversation.context_type.to_string(),
                    context_id: conversation.context_id.clone(),
                    role: created.role.to_string(),
                    content: created.content,
                    created_at: Some(created.created_at.to_rfc3339()),
                    metadata: created.metadata,
                },
            );
        }
    }

    Ok(created_count)
}

async fn remove_invalid_bridge_messages(
    state: &AppState,
    conversation_id: &ChatConversationId,
    workspace: &AgentConversationWorkspace,
    messages: Vec<ChatMessage>,
) -> AppResult<(HashSet<String>, usize)> {
    let expected_session_id = workspace
        .linked_ideation_session_id
        .as_ref()
        .map(|id| id.as_str().to_string());
    let mut valid_bridge_keys = HashSet::new();
    let mut removed = 0;

    for message in messages {
        let Some(metadata) = parse_bridge_metadata(&message) else {
            continue;
        };
        let source_session_id = metadata
            .get("source_session_id")
            .or_else(|| metadata.get("sourceSessionId"))
            .and_then(JsonValue::as_str);
        let is_valid_owner = expected_session_id.as_deref() == source_session_id;
        if !is_valid_owner {
            state.chat_message_repo.delete(&message.id).await?;
            removed += 1;
            continue;
        }

        if let Some(event_key) = metadata
            .get("bridge_event_key")
            .or_else(|| metadata.get("bridgeEventKey"))
            .and_then(JsonValue::as_str)
        {
            valid_bridge_keys.insert(event_key.to_string());
        }
    }

    if removed > 0 {
        tracing::warn!(
            conversation_id = %conversation_id,
            removed,
            "Removed agent workspace bridge messages that did not match the workspace's linked ideation session"
        );
    }

    Ok((valid_bridge_keys, removed))
}

async fn create_bridge_chat_message(
    state: &AppState,
    conversation: &crate::domain::entities::ChatConversation,
    conversation_id: &ChatConversationId,
    source_session_id: &str,
    bridge_message: AgentWorkspaceBridgeMessage,
) -> AppResult<ChatMessage> {
    let mut metadata = JsonMap::new();
    metadata.insert(
        "source".to_string(),
        JsonValue::String(BRIDGE_SOURCE.to_string()),
    );
    metadata.insert(
        "bridge_event_key".to_string(),
        JsonValue::String(bridge_message.event_key.clone()),
    );
    metadata.insert(
        "bridge_event_type".to_string(),
        JsonValue::String(bridge_message.event_type),
    );
    metadata.insert(
        "source_session_id".to_string(),
        JsonValue::String(source_session_id.to_string()),
    );
    metadata.insert("payload".to_string(), bridge_message.metadata);

    let mut message = create_assistant_message(
        conversation.context_type,
        &conversation.context_id,
        &bridge_message.content,
        *conversation_id,
        &[],
        &[],
    );
    message.metadata = Some(JsonValue::Object(metadata).to_string());
    message.created_at = bridge_message.created_at;

    state.chat_message_repo.create(message).await
}

async fn refresh_conversation_stats(
    state: &AppState,
    conversation_id: &ChatConversationId,
) -> AppResult<()> {
    let messages = state
        .chat_message_repo
        .get_by_conversation(conversation_id)
        .await?;
    if let Some(last_message) = messages.last() {
        state
            .chat_conversation_repo
            .update_message_stats(
                conversation_id,
                messages.len() as i64,
                last_message.created_at,
            )
            .await?;
    }
    Ok(())
}

fn parse_bridge_metadata(message: &ChatMessage) -> Option<JsonMap<String, JsonValue>> {
    let metadata = message.metadata.as_deref()?;
    let JsonValue::Object(parsed) = serde_json::from_str::<JsonValue>(metadata).ok()? else {
        return None;
    };
    let source = parsed
        .get("source")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    (source == BRIDGE_SOURCE).then_some(parsed)
}

pub fn bridge_message_from_external_event(
    event: &ExternalEventRecord,
    session_id: &str,
) -> Option<AgentWorkspaceBridgeMessage> {
    let payload = parse_event_payload(event)?;
    if payload_session_id(&payload).as_deref() != Some(session_id) {
        return None;
    }
    let created_at = parse_event_created_at(&event.created_at);

    match event.event_type.as_str() {
        "ideation:plan_created" => {
            let title = string_field(&payload, "plan_title")
                .or_else(|| string_field(&payload, "session_title"));
            Some(AgentWorkspaceBridgeMessage {
                event_key: format!("ideation:{session_id}:plan_created"),
                event_type: event.event_type.clone(),
                content: title
                    .map(|title| format!("Plan is ready: {title}."))
                    .unwrap_or_else(|| "Plan is ready in the attached ideation run.".to_string()),
                metadata: external_event_metadata(event, payload),
                created_at,
            })
        }
        "ideation:verified" => Some(AgentWorkspaceBridgeMessage {
            event_key: format!("ideation:{session_id}:verified"),
            event_type: event.event_type.clone(),
            content: "Plan verified for the attached ideation run.".to_string(),
            metadata: external_event_metadata(event, payload),
            created_at,
        }),
        "ideation:proposals_ready" => {
            let count = number_field(&payload, "proposal_count");
            Some(AgentWorkspaceBridgeMessage {
                event_key: format!("ideation:{session_id}:proposals_ready"),
                event_type: event.event_type.clone(),
                content: match count {
                    Some(1) => "Proposals are ready: 1 proposal generated.".to_string(),
                    Some(count) => {
                        format!("Proposals are ready: {count} proposals generated.")
                    }
                    None => "Proposals are ready: multiple proposals generated.".to_string(),
                },
                metadata: external_event_metadata(event, payload),
                created_at,
            })
        }
        "ideation:session_accepted" => Some(AgentWorkspaceBridgeMessage {
            event_key: format!("ideation:{session_id}:session_accepted"),
            event_type: event.event_type.clone(),
            content: "Plan accepted. RalphX is creating and running implementation tasks."
                .to_string(),
            metadata: external_event_metadata(event, payload),
            created_at,
        }),
        "task:execution_started" => Some(AgentWorkspaceBridgeMessage {
            event_key: format!(
                "pipeline:{session_id}:task_execution_started:{}",
                task_identity(&payload)
            ),
            event_type: event.event_type.clone(),
            content: format!("Task started: {}.", task_label(&payload)),
            metadata: external_event_metadata(event, payload),
            created_at,
        }),
        "task:execution_completed" => Some(AgentWorkspaceBridgeMessage {
            event_key: format!(
                "pipeline:{session_id}:task_execution_completed:{}",
                task_identity(&payload)
            ),
            event_type: event.event_type.clone(),
            content: format!("Task execution completed: {}.", task_label(&payload)),
            metadata: external_event_metadata(event, payload),
            created_at,
        }),
        "merge:ready" => Some(AgentWorkspaceBridgeMessage {
            event_key: format!(
                "pipeline:{session_id}:merge_ready:{}",
                task_identity(&payload)
            ),
            event_type: event.event_type.clone(),
            content: format!("Merge ready: {}.", task_label(&payload)),
            metadata: external_event_metadata(event, payload),
            created_at,
        }),
        "merge:completed" => {
            let commit = string_field(&payload, "commit_sha");
            let suffix = commit
                .as_deref()
                .map(|sha| format!(" Commit {}.", sha.chars().take(7).collect::<String>()))
                .unwrap_or_default();
            Some(AgentWorkspaceBridgeMessage {
                event_key: format!(
                    "pipeline:{session_id}:merge_completed:{}:{}",
                    task_identity(&payload),
                    commit.unwrap_or_else(|| event.id.to_string())
                ),
                event_type: event.event_type.clone(),
                content: format!("Merged: {}.{suffix}", task_label(&payload)),
                metadata: external_event_metadata(event, payload),
                created_at,
            })
        }
        "task:status_changed" => {
            let new_status = string_field(&payload, "new_status")?;
            if !["blocked", "failed", "merge_incomplete", "cancelled"]
                .contains(&new_status.as_str())
            {
                return None;
            }
            Some(AgentWorkspaceBridgeMessage {
                event_key: format!(
                    "pipeline:{session_id}:task_status:{}:{new_status}",
                    task_identity(&payload)
                ),
                event_type: event.event_type.clone(),
                content: format!(
                    "Task needs attention: {} is {}.",
                    task_label(&payload),
                    new_status.replace('_', " ")
                ),
                metadata: external_event_metadata(event, payload),
                created_at,
            })
        }
        _ => None,
    }
}

fn parse_event_payload(event: &ExternalEventRecord) -> Option<JsonMap<String, JsonValue>> {
    let JsonValue::Object(payload) = serde_json::from_str::<JsonValue>(&event.payload).ok()? else {
        return None;
    };
    Some(payload)
}

fn external_event_metadata(
    event: &ExternalEventRecord,
    payload: JsonMap<String, JsonValue>,
) -> JsonValue {
    let mut metadata = JsonMap::new();
    metadata.insert(
        "externalEventId".to_string(),
        JsonValue::Number(event.id.into()),
    );
    metadata.insert("payload".to_string(), JsonValue::Object(payload));
    JsonValue::Object(metadata)
}

fn parse_event_created_at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn payload_session_id(payload: &JsonMap<String, JsonValue>) -> Option<String> {
    string_field(payload, "session_id").or_else(|| string_field(payload, "sessionId"))
}

fn task_identity(payload: &JsonMap<String, JsonValue>) -> String {
    string_field(payload, "task_id")
        .or_else(|| string_field(payload, "taskId"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn task_label(payload: &JsonMap<String, JsonValue>) -> String {
    let label = string_field(payload, "task_title")
        .or_else(|| string_field(payload, "taskTitle"))
        .or_else(|| string_field(payload, "human_context").map(clean_human_context))
        .unwrap_or_else(|| task_identity(payload));
    truncate_label(&label)
}

fn clean_human_context(value: String) -> String {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') {
        return trimmed.to_string();
    }
    trimmed
        .find(']')
        .map(|index| trimmed[index + 1..].trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| trimmed.to_string())
}

fn truncate_label(value: &str) -> String {
    const MAX_LABEL_LEN: usize = 140;
    if value.chars().count() <= MAX_LABEL_LEN {
        return value.to_string();
    }
    let prefix: String = value.chars().take(MAX_LABEL_LEN - 3).collect();
    format!("{prefix}...")
}

fn string_field(payload: &JsonMap<String, JsonValue>, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn number_field(payload: &JsonMap<String, JsonValue>, key: &str) -> Option<i64> {
    payload.get(key).and_then(JsonValue::as_i64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{
        AgentConversationWorkspaceMode, ChatConversation, IdeationAnalysisBaseRefKind,
        IdeationSessionId, ProjectId,
    };

    fn event(id: i64, event_type: &str, payload: JsonValue) -> ExternalEventRecord {
        ExternalEventRecord {
            id,
            event_type: event_type.to_string(),
            project_id: "project-1".to_string(),
            payload: payload.to_string(),
            created_at: "2026-04-27T20:39:07Z".to_string(),
        }
    }

    async fn create_workspace(
        state: &AppState,
        project_id: ProjectId,
        title: &str,
        linked_session_id: Option<&str>,
    ) -> ChatConversationId {
        let mut conversation = ChatConversation::new_project(project_id.clone());
        conversation.title = Some(title.to_string());
        let conversation_id = conversation.id;
        state
            .chat_conversation_repo
            .create(conversation)
            .await
            .unwrap();

        let mut workspace = AgentConversationWorkspace::new(
            conversation_id,
            project_id,
            if linked_session_id.is_some() {
                AgentConversationWorkspaceMode::Ideation
            } else {
                AgentConversationWorkspaceMode::Edit
            },
            IdeationAnalysisBaseRefKind::CurrentBranch,
            "main".to_string(),
            Some("Current branch".to_string()),
            None,
            format!("agent-{conversation_id}"),
            "/tmp/agent-workspace".to_string(),
        );
        workspace.linked_ideation_session_id =
            linked_session_id.map(|id| IdeationSessionId::from_string(id.to_string()));
        state
            .agent_conversation_workspace_repo
            .create_or_update(workspace)
            .await
            .unwrap();
        conversation_id
    }

    #[test]
    fn maps_external_events_to_backend_bridge_messages() {
        let message = bridge_message_from_external_event(
            &event(
                10,
                "ideation:plan_created",
                json!({
                    "session_id": "session-1",
                    "plan_title": "Fix Font Scale Switching Regression"
                }),
            ),
            "session-1",
        )
        .unwrap();

        assert_eq!(message.event_key, "ideation:session-1:plan_created");
        assert_eq!(
            message.content,
            "Plan is ready: Fix Font Scale Switching Regression."
        );
    }

    #[tokio::test]
    async fn reconciles_only_the_workspace_linked_to_the_ideation_session() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let linked_conversation_id = create_workspace(
            &state,
            project_id.clone(),
            "Linked workspace",
            Some("session-1"),
        )
        .await;
        let edit_conversation_id =
            create_workspace(&state, project_id.clone(), "Edit workspace", None).await;

        state
            .external_events_repo
            .insert_event(
                "ideation:verified",
                project_id.as_str(),
                &json!({ "session_id": "session-1" }).to_string(),
            )
            .await
            .unwrap();

        let linked_created =
            reconcile_agent_workspace_bridge_messages(&state, &linked_conversation_id, None)
                .await
                .unwrap();
        let edit_created =
            reconcile_agent_workspace_bridge_messages(&state, &edit_conversation_id, None)
                .await
                .unwrap();

        assert_eq!(linked_created, 1);
        assert_eq!(edit_created, 0);
        let linked_messages = state
            .chat_message_repo
            .get_by_conversation(&linked_conversation_id)
            .await
            .unwrap();
        let edit_messages = state
            .chat_message_repo
            .get_by_conversation(&edit_conversation_id)
            .await
            .unwrap();
        assert_eq!(linked_messages.len(), 1);
        assert_eq!(
            linked_messages[0].content,
            "Plan verified for the attached ideation run."
        );
        assert!(edit_messages.is_empty());
    }

    #[tokio::test]
    async fn removes_existing_bridge_messages_that_do_not_match_workspace_link() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let conversation_id = create_workspace(&state, project_id, "Edit workspace", None).await;

        let mut bad_message = create_assistant_message(
            ChatContextType::Project,
            "project-1",
            "Plan verified for the attached ideation run.",
            conversation_id,
            &[],
            &[],
        );
        bad_message.metadata = Some(
            json!({
                "source": BRIDGE_SOURCE,
                "bridge_event_key": "ideation:session-1:verified",
                "source_session_id": "session-1"
            })
            .to_string(),
        );
        state.chat_message_repo.create(bad_message).await.unwrap();

        let created = reconcile_agent_workspace_bridge_messages(&state, &conversation_id, None)
            .await
            .unwrap();
        let messages = state
            .chat_message_repo
            .get_by_conversation(&conversation_id)
            .await
            .unwrap();

        assert_eq!(created, 0);
        assert!(messages.is_empty());
    }
}
