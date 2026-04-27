use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::application::chat_service::{ChatService, SendCallerContext, SendMessageOptions};
use crate::application::AppState;
use crate::domain::entities::{
    AgentConversationWorkspace, ChatContextType, ChatConversationId, ChatMessage,
};
use crate::domain::repositories::external_events_repository::ExternalEventRecord;
use crate::error::{AppError, AppResult};

const LEGACY_BRIDGE_SOURCE: &str = "project_agent_ideation_bridge";
const WAKEUP_SOURCE: &str = "project_agent_workspace_bridge_wakeup";
const MAX_BRIDGE_EVENT_REPLAY: i64 = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWorkspaceBridgeEvent {
    pub event_key: String,
    pub event_type: String,
    pub external_event_id: i64,
    pub payload: JsonMap<String, JsonValue>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWorkspaceBridgeWakeUp {
    pub conversation_id: ChatConversationId,
    pub project_id: String,
    pub source_session_id: String,
    pub event_keys: Vec<String>,
    pub content: String,
    pub metadata: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentWorkspaceBridgeWakeUpResult {
    pub conversation_id: String,
    pub agent_run_id: String,
    pub was_queued: bool,
    pub queued_message_id: Option<String>,
    pub event_count: usize,
}

pub async fn wake_agent_workspace_for_bridge_events<S: ChatService + ?Sized>(
    state: &AppState,
    chat_service: &S,
    conversation_id: &ChatConversationId,
) -> AppResult<Option<AgentWorkspaceBridgeWakeUpResult>> {
    let Some(wake_up) = prepare_agent_workspace_bridge_wakeup(state, conversation_id).await? else {
        return Ok(None);
    };
    let event_count = wake_up.event_keys.len();
    let result = chat_service
        .send_message(
            ChatContextType::Project,
            &wake_up.project_id,
            &wake_up.content,
            SendMessageOptions {
                metadata: Some(wake_up.metadata),
                conversation_id_override: Some(wake_up.conversation_id),
                caller_context: SendCallerContext::DrainService,
                ..Default::default()
            },
        )
        .await
        .map_err(|error| AppError::Agent(error.to_string()))?;

    Ok(Some(AgentWorkspaceBridgeWakeUpResult {
        conversation_id: result.conversation_id,
        agent_run_id: result.agent_run_id,
        was_queued: result.was_queued,
        queued_message_id: result.queued_message_id,
        event_count,
    }))
}

pub async fn prepare_agent_workspace_bridge_wakeup(
    state: &AppState,
    conversation_id: &ChatConversationId,
) -> AppResult<Option<AgentWorkspaceBridgeWakeUp>> {
    let Some(conversation) = state
        .chat_conversation_repo
        .get_by_id(conversation_id)
        .await?
    else {
        return Ok(None);
    };
    if conversation.context_type != ChatContextType::Project {
        return Ok(None);
    }

    let Some(workspace) = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(conversation_id)
        .await?
    else {
        return Ok(None);
    };

    let existing_messages = state
        .chat_message_repo
        .get_by_conversation(conversation_id)
        .await?;
    let (mut delivered_event_keys, removed_invalid_count) =
        reconcile_legacy_bridge_messages(state, conversation_id, &workspace, existing_messages)
            .await?;
    if removed_invalid_count > 0 {
        refresh_conversation_stats(state, conversation_id).await?;
    }
    collect_queued_bridge_event_keys(state, conversation_id, &mut delivered_event_keys);

    let Some(session_id) = workspace.linked_ideation_session_id.as_ref() else {
        return Ok(None);
    };
    let events = state
        .external_events_repo
        .get_events_after_cursor(
            &[workspace.project_id.as_str().to_string()],
            0,
            MAX_BRIDGE_EVENT_REPLAY,
        )
        .await?;

    let new_events: Vec<_> = events
        .into_iter()
        .filter_map(|event| bridge_event_from_external_event(&event, session_id.as_str()))
        .filter(|event| !delivered_event_keys.contains(&event.event_key))
        .collect();

    if new_events.is_empty() {
        return Ok(None);
    }

    Ok(Some(build_wakeup(
        *conversation_id,
        workspace.project_id.as_str().to_string(),
        session_id.as_str().to_string(),
        new_events,
    )))
}

async fn reconcile_legacy_bridge_messages(
    state: &AppState,
    conversation_id: &ChatConversationId,
    workspace: &AgentConversationWorkspace,
    messages: Vec<ChatMessage>,
) -> AppResult<(HashSet<String>, usize)> {
    let expected_session_id = workspace
        .linked_ideation_session_id
        .as_ref()
        .map(|id| id.as_str().to_string());
    let mut delivered_event_keys = HashSet::new();
    let mut removed = 0;

    for message in messages {
        let Some(metadata) = parse_message_metadata(&message) else {
            continue;
        };

        if metadata_source(&metadata) == Some(WAKEUP_SOURCE) {
            collect_wakeup_event_keys(&metadata, &mut delivered_event_keys);
            continue;
        }

        if metadata_source(&metadata) != Some(LEGACY_BRIDGE_SOURCE) {
            continue;
        }

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
            delivered_event_keys.insert(event_key.to_string());
        }
    }

    if removed > 0 {
        tracing::warn!(
            conversation_id = %conversation_id,
            removed,
            "Removed agent workspace bridge messages that did not match the workspace's linked ideation session"
        );
    }

    Ok((delivered_event_keys, removed))
}

fn collect_queued_bridge_event_keys(
    state: &AppState,
    conversation_id: &ChatConversationId,
    delivered_event_keys: &mut HashSet<String>,
) {
    let queue_context_id = conversation_id.as_str();
    for queued in state
        .message_queue
        .get_queued(ChatContextType::Project, &queue_context_id)
    {
        let Some(metadata) = queued.metadata_override.as_deref() else {
            continue;
        };
        let Ok(JsonValue::Object(metadata)) = serde_json::from_str::<JsonValue>(metadata) else {
            continue;
        };
        if metadata_source(&metadata) == Some(WAKEUP_SOURCE) {
            collect_wakeup_event_keys(&metadata, delivered_event_keys);
        }
    }
}

fn collect_wakeup_event_keys(
    metadata: &JsonMap<String, JsonValue>,
    delivered_event_keys: &mut HashSet<String>,
) {
    let Some(keys) = metadata
        .get("bridge_event_keys")
        .or_else(|| metadata.get("bridgeEventKeys"))
        .and_then(JsonValue::as_array)
    else {
        return;
    };
    delivered_event_keys.extend(
        keys.iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_string),
    );
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

fn parse_message_metadata(message: &ChatMessage) -> Option<JsonMap<String, JsonValue>> {
    let metadata = message.metadata.as_deref()?;
    let JsonValue::Object(parsed) = serde_json::from_str::<JsonValue>(metadata).ok()? else {
        return None;
    };
    Some(parsed)
}

fn metadata_source(metadata: &JsonMap<String, JsonValue>) -> Option<&str> {
    metadata.get("source").and_then(JsonValue::as_str)
}

pub fn bridge_event_from_external_event(
    event: &ExternalEventRecord,
    session_id: &str,
) -> Option<AgentWorkspaceBridgeEvent> {
    let payload = parse_event_payload(event)?;
    if payload_session_id(&payload).as_deref() != Some(session_id) {
        return None;
    }
    let event_key = bridge_event_key(event, session_id, &payload)?;

    Some(AgentWorkspaceBridgeEvent {
        event_key,
        event_type: event.event_type.clone(),
        external_event_id: event.id,
        payload,
        created_at: parse_event_created_at(&event.created_at),
    })
}

fn bridge_event_key(
    event: &ExternalEventRecord,
    session_id: &str,
    payload: &JsonMap<String, JsonValue>,
) -> Option<String> {
    match event.event_type.as_str() {
        "ideation:plan_created" => Some(format!("ideation:{session_id}:plan_created")),
        "ideation:verified" => Some(format!("ideation:{session_id}:verified")),
        "ideation:proposals_ready" => Some(format!("ideation:{session_id}:proposals_ready")),
        "ideation:session_accepted" => Some(format!("ideation:{session_id}:session_accepted")),
        "task:execution_started" => Some(format!(
            "pipeline:{session_id}:task_execution_started:{}",
            task_identity(payload)
        )),
        "task:execution_completed" => Some(format!(
            "pipeline:{session_id}:task_execution_completed:{}",
            task_identity(payload)
        )),
        "merge:ready" => Some(format!(
            "pipeline:{session_id}:merge_ready:{}",
            task_identity(payload)
        )),
        "merge:completed" => {
            let commit =
                string_field(payload, "commit_sha").unwrap_or_else(|| event.id.to_string());
            Some(format!(
                "pipeline:{session_id}:merge_completed:{}:{commit}",
                task_identity(payload)
            ))
        }
        "task:status_changed" => {
            let new_status = string_field(payload, "new_status")?;
            if !["blocked", "failed", "merge_incomplete", "cancelled"]
                .contains(&new_status.as_str())
            {
                return None;
            }
            Some(format!(
                "pipeline:{session_id}:task_status:{}:{new_status}",
                task_identity(payload)
            ))
        }
        _ => None,
    }
}

fn build_wakeup(
    conversation_id: ChatConversationId,
    project_id: String,
    source_session_id: String,
    events: Vec<AgentWorkspaceBridgeEvent>,
) -> AgentWorkspaceBridgeWakeUp {
    let event_keys: Vec<_> = events.iter().map(|event| event.event_key.clone()).collect();
    let event_payload = JsonValue::Array(
        events
            .iter()
            .map(|event| {
                json!({
                    "bridge_event_key": event.event_key,
                    "external_event_id": event.external_event_id,
                    "event_type": event.event_type,
                    "created_at": event.created_at.to_rfc3339(),
                    "payload": event.payload,
                })
            })
            .collect(),
    );
    let payload_text =
        serde_json::to_string_pretty(&event_payload).unwrap_or_else(|_| event_payload.to_string());
    let event_word = if events.len() == 1 { "event" } else { "events" };
    let content = format!(
        "RalphX workflow {event_word} arrived for this agent workspace. Review the payload, explain what changed, and take action with your tools if the workspace needs intervention. If no action is needed, keep the response brief.\n\n```json\n{payload_text}\n```"
    );
    let metadata = json!({
        "source": WAKEUP_SOURCE,
        "source_session_id": source_session_id,
        "bridge_event_keys": event_keys,
        "event_count": events.len(),
    })
    .to_string();

    AgentWorkspaceBridgeWakeUp {
        conversation_id,
        project_id,
        source_session_id,
        event_keys,
        content,
        metadata,
    }
}

fn parse_event_payload(event: &ExternalEventRecord) -> Option<JsonMap<String, JsonValue>> {
    let JsonValue::Object(payload) = serde_json::from_str::<JsonValue>(&event.payload).ok()? else {
        return None;
    };
    Some(payload)
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

fn string_field(payload: &JsonMap<String, JsonValue>, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::application::chat_service::create_assistant_message;
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
    fn maps_external_events_to_backend_bridge_event_keys() {
        let event = bridge_event_from_external_event(
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

        assert_eq!(event.event_key, "ideation:session-1:plan_created");
        assert_eq!(event.event_type, "ideation:plan_created");
    }

    #[tokio::test]
    async fn prepares_one_workspace_agent_wakeup_for_linked_events() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let conversation_id = create_workspace(
            &state,
            project_id.clone(),
            "Linked workspace",
            Some("session-1"),
        )
        .await;

        state
            .external_events_repo
            .insert_event(
                "ideation:verified",
                project_id.as_str(),
                &json!({ "session_id": "session-1", "gap_score": 1 }).to_string(),
            )
            .await
            .unwrap();

        let wakeup = prepare_agent_workspace_bridge_wakeup(&state, &conversation_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(wakeup.event_keys, vec!["ideation:session-1:verified"]);
        assert!(wakeup.content.contains("RalphX workflow event arrived"));
        assert!(wakeup.content.contains("\"gap_score\": 1"));
        let metadata: JsonValue = serde_json::from_str(&wakeup.metadata).unwrap();
        assert_eq!(metadata["source"], WAKEUP_SOURCE);

        let messages = state
            .chat_message_repo
            .get_by_conversation(&conversation_id)
            .await
            .unwrap();
        assert!(
            messages.is_empty(),
            "bridge preparation must not write assistant messages"
        );
    }

    #[tokio::test]
    async fn does_not_prepare_wakeup_for_unlinked_edit_workspace() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let conversation_id =
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

        let wakeup = prepare_agent_workspace_bridge_wakeup(&state, &conversation_id)
            .await
            .unwrap();

        assert!(wakeup.is_none());
    }

    #[tokio::test]
    async fn skips_events_already_persisted_as_workspace_wakeups() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let conversation_id = create_workspace(
            &state,
            project_id.clone(),
            "Linked workspace",
            Some("session-1"),
        )
        .await;

        let mut wakeup_message = create_assistant_message(
            ChatContextType::Project,
            project_id.as_str(),
            "RalphX workflow event arrived.",
            conversation_id,
            &[],
            &[],
        );
        wakeup_message.metadata = Some(
            json!({
                "source": WAKEUP_SOURCE,
                "bridge_event_keys": ["ideation:session-1:verified"]
            })
            .to_string(),
        );
        state
            .chat_message_repo
            .create(wakeup_message)
            .await
            .unwrap();
        state
            .external_events_repo
            .insert_event(
                "ideation:verified",
                project_id.as_str(),
                &json!({ "session_id": "session-1" }).to_string(),
            )
            .await
            .unwrap();

        let wakeup = prepare_agent_workspace_bridge_wakeup(&state, &conversation_id)
            .await
            .unwrap();

        assert!(wakeup.is_none());
    }

    #[tokio::test]
    async fn skips_events_already_queued_as_workspace_wakeups() {
        let state = AppState::new_test();
        let project_id = ProjectId::from_string("project-1".to_string());
        let conversation_id = create_workspace(
            &state,
            project_id.clone(),
            "Linked workspace",
            Some("session-1"),
        )
        .await;

        state.message_queue.queue_with_overrides(
            ChatContextType::Project,
            conversation_id.as_str(),
            "queued wakeup".to_string(),
            Some(
                json!({
                    "source": WAKEUP_SOURCE,
                    "bridge_event_keys": ["ideation:session-1:verified"]
                })
                .to_string(),
            ),
            None,
            None,
        );
        state
            .external_events_repo
            .insert_event(
                "ideation:verified",
                project_id.as_str(),
                &json!({ "session_id": "session-1" }).to_string(),
            )
            .await
            .unwrap();

        let wakeup = prepare_agent_workspace_bridge_wakeup(&state, &conversation_id)
            .await
            .unwrap();

        assert!(wakeup.is_none());
    }

    #[tokio::test]
    async fn removes_existing_legacy_bridge_messages_that_do_not_match_workspace_link() {
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
                "source": LEGACY_BRIDGE_SOURCE,
                "bridge_event_key": "ideation:session-1:verified",
                "source_session_id": "session-1"
            })
            .to_string(),
        );
        state.chat_message_repo.create(bad_message).await.unwrap();

        let wakeup = prepare_agent_workspace_bridge_wakeup(&state, &conversation_id)
            .await
            .unwrap();
        let messages = state
            .chat_message_repo
            .get_by_conversation(&conversation_id)
            .await
            .unwrap();

        assert!(wakeup.is_none());
        assert!(messages.is_empty());
    }
}
