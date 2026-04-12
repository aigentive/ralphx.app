use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, os::unix::fs::PermissionsExt};

use axum::{extract::State, Json};
use ralphx_lib::application::{AppState, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::agents::AgentHarnessKind;
use ralphx_lib::domain::entities::{ChatConversation, DelegatedSessionId, IdeationSession, Project};
use ralphx_lib::http_server::handlers::{cancel_delegate, start_delegate, wait_delegate};
use ralphx_lib::http_server::types::{
    DelegateCancelRequest, DelegateStartRequest, DelegateWaitRequest, HttpServerState,
};
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a repo-root parent")
        .to_path_buf()
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = self.original.as_ref() {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn install_fake_codex_cli() -> (TempDir, PathBuf) {
    let tempdir = TempDir::new().expect("tempdir");
    let script_path = tempdir.path().join("codex");
    let script = r#"#!/bin/sh
if [ "$1" = "--help" ]; then
cat <<'EOF'
Codex CLI

Commands:
  exec        Run Codex non-interactively [aliases: e]
  mcp         Manage external MCP servers for Codex
  resume      Resume a previous interactive session

Options:
  -c, --config <key=value>
  -m, --model <MODEL>
  -s, --sandbox <SANDBOX_MODE>
      --search
      --add-dir <DIR>
EOF
exit 0
fi

if [ "$1" = "--version" ]; then
echo "codex-cli 0.116.0"
exit 0
fi

if [ "$1" = "exec" ] && [ "$2" = "--help" ]; then
cat <<'EOF'
Run Codex non-interactively

Usage: codex exec [OPTIONS] [PROMPT] [COMMAND]

Options:
  -c, --config <key=value>
  -m, --model <MODEL>
  -s, --sandbox <SANDBOX_MODE>
      --add-dir <DIR>
      --json
  -C, --cd <DIR>
      --skip-git-repo-check
EOF
exit 0
fi

if [ "$1" = "exec" ]; then
printf '%s\n' '{"type":"thread.started","thread_id":"delegation-thread-1"}'
printf '%s\n' '{"type":"item.completed","item":{"type":"agent_message","text":"MOCK_COMPLETION"}}'
printf '%s\n' '{"type":"turn.completed","usage":{"input_tokens":11,"cached_input_tokens":2,"output_tokens":7}}'
exit 0
fi

echo "unsupported invocation" >&2
exit 2
"#;
    fs::write(&script_path, script).expect("write fake codex cli");
    let mut permissions = fs::metadata(&script_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).expect("chmod fake codex cli");
    (tempdir, script_path)
}

fn build_state(app_state: Arc<AppState>) -> HttpServerState {
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
        delegation_service: Default::default(),
    }
}

async fn create_parent_session(state: &HttpServerState) -> IdeationSession {
    let project = Project::new(
        "Delegation Test Project".to_string(),
        repo_root().display().to_string(),
    );
    let project_id = project.id.clone();
    state.app_state.project_repo.create(project).await.unwrap();

    let session = IdeationSession::builder()
        .project_id(project_id)
        .title("Delegation Parent")
        .cross_project_checked(true)
        .build();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_delegate_start_creates_delegated_session_and_completes_with_mock_client() {
    let (_fake_codex_dir, fake_codex_path) = install_fake_codex_cli();
    let _codex_cli_guard = EnvVarGuard::set(
        "CODEX_CLI_PATH",
        fake_codex_path.to_str().expect("fake codex path utf8"),
    );
    let app_state = Arc::new(AppState::new_sqlite_test());
    let state = build_state(app_state);
    let parent = create_parent_session(&state).await;
    let parent_conversation = state
        .app_state
        .chat_conversation_repo
        .create(ChatConversation::new_ideation(parent.id.clone()))
        .await
        .unwrap();
    let parent_conversation_id = parent_conversation.id.as_str();

    let start = start_delegate(
        State(state.clone()),
        Json(DelegateStartRequest {
            parent_session_id: parent.id.as_str().to_string(),
            parent_turn_id: Some("turn-42".to_string()),
            parent_message_id: Some("msg-99".to_string()),
            parent_conversation_id: None,
            parent_tool_use_id: Some("toolu-parent-1".to_string()),
            delegated_session_id: None,
            child_session_id: None,
            agent_name: "ralphx-ideation".to_string(),
            prompt: "Review the proposal set and summarize the main implementation risks."
                .to_string(),
            title: Some("Delegated Risk Review".to_string()),
            inherit_context: true,
            harness: Some(AgentHarnessKind::Codex),
            model: None,
            logical_effort: None,
            approval_policy: None,
            sandbox_mode: None,
        }),
    )
    .await
    .unwrap()
    .0;

    assert_eq!(start.parent_context_type, "ideation");
    assert_eq!(start.parent_context_id, parent.id.as_str());
    assert_eq!(start.parent_turn_id.as_deref(), Some("turn-42"));
    assert_eq!(start.parent_message_id.as_deref(), Some("msg-99"));
    assert_eq!(start.parent_tool_use_id.as_deref(), Some("toolu-parent-1"));
    assert_eq!(start.agent_name, "ralphx-ideation");
    assert_eq!(start.harness, "codex");
    assert_eq!(start.status, "running");
    assert_ne!(start.delegated_session_id, parent.id.as_str());
    assert_eq!(
        start.parent_conversation_id.as_deref(),
        Some(parent_conversation_id.as_str())
    );
    assert!(start.delegated_conversation_id.is_some());
    assert!(start.delegated_agent_run_id.is_some());
    assert_eq!(start.history.len(), 1);
    assert_eq!(start.history[0].status, "running");

    let delegated_id = DelegatedSessionId::from_string(start.delegated_session_id.clone());
    let delegated = state
        .app_state
        .delegated_session_repo
        .get_by_id(&delegated_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delegated.parent_context_type, "ideation");
    assert_eq!(delegated.parent_context_id, parent.id.as_str());
    assert_eq!(delegated.status, "running");

    let waited = {
        let mut snapshot = None;
        for _ in 0..20 {
            let candidate = wait_delegate(
                State(state.clone()),
                Json(DelegateWaitRequest {
                    job_id: start.job_id.clone(),
                    include_delegated_status: Some(true),
                    include_child_status: None,
                    include_messages: Some(false),
                    message_limit: None,
                }),
            )
            .await
            .unwrap()
            .0;
            if candidate.status != "running" {
                snapshot = Some(candidate);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        snapshot.expect("delegation job should settle")
    };

    assert_eq!(waited.job_id, start.job_id);
    assert_eq!(waited.status, "completed");
    assert_eq!(waited.content.as_deref(), Some("MOCK_COMPLETION"));
    assert!(waited.error.is_none());
    assert_eq!(waited.parent_turn_id.as_deref(), Some("turn-42"));
    assert_eq!(waited.parent_message_id.as_deref(), Some("msg-99"));
    assert_eq!(waited.parent_tool_use_id.as_deref(), Some("toolu-parent-1"));
    assert_eq!(
        waited.history.iter().map(|entry| entry.status.as_str()).collect::<Vec<_>>(),
        vec!["running", "completed"]
    );
    let delegated_status = waited
        .delegated_status
        .expect("delegated status should be hydrated");
    assert_eq!(delegated_status.session.id, waited.delegated_session_id);
    assert_eq!(delegated_status.session.parent_context_type, "ideation");
    assert_eq!(delegated_status.session.parent_context_id, parent.id.as_str());
    assert_eq!(delegated_status.session.status, "completed");
    assert_eq!(delegated_status.agent_state.estimated_status, "idle");
    assert_eq!(
        delegated_status.conversation_id.as_deref(),
        waited.delegated_conversation_id.as_deref()
    );
    let latest_run = delegated_status.latest_run.expect("latest delegated run");
    assert_eq!(
        Some(latest_run.agent_run_id.as_str()),
        waited.delegated_agent_run_id.as_deref()
    );
    assert_eq!(latest_run.status, "completed");
    assert_eq!(latest_run.harness.as_deref(), Some("codex"));
    assert_eq!(latest_run.upstream_provider.as_deref(), Some("openai"));
    assert!(delegated_status.recent_messages.is_none());

    let delegated_after = state
        .app_state
        .delegated_session_repo
        .get_by_id(&delegated_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delegated_after.status, "completed");
    assert!(delegated_after.completed_at.is_some());

    assert_eq!(latest_run.input_tokens, Some(11));
    assert_eq!(latest_run.cache_read_tokens, Some(2));
    assert_eq!(latest_run.output_tokens, Some(7));
}

#[tokio::test]
async fn test_delegate_start_rejects_unknown_agent_name() {
    let state = build_state(Arc::new(AppState::new_sqlite_test()));
    let parent = create_parent_session(&state).await;

    let error = start_delegate(
        State(state),
        Json(DelegateStartRequest {
            parent_session_id: parent.id.as_str().to_string(),
            parent_turn_id: Some("turn-bad".to_string()),
            parent_message_id: Some("msg-bad".to_string()),
            parent_conversation_id: None,
            parent_tool_use_id: None,
            delegated_session_id: None,
            child_session_id: None,
            agent_name: "ralphx-does-not-exist".to_string(),
            prompt: "noop".to_string(),
            title: None,
            inherit_context: true,
            harness: Some(AgentHarnessKind::Codex),
            model: None,
            logical_effort: None,
            approval_policy: None,
            sandbox_mode: None,
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(
        error.1 .0["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Unknown canonical agent")
    );
}

#[tokio::test]
async fn test_delegate_cancel_rejects_unknown_job() {
    let state = build_state(Arc::new(AppState::new_sqlite_test()));

    let error = cancel_delegate(
        State(state),
        Json(DelegateCancelRequest {
            job_id: "missing-job".to_string(),
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::NOT_FOUND);
}
