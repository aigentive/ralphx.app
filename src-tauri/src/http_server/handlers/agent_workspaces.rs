//! Agent workspace HTTP handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::*;
use crate::application::agent_conversation_workspace::resolve_valid_agent_conversation_workspace_path;
use crate::application::publish_resilience::{
    inspect_publish_branch_freshness_for_source, verify_agent_workspace_repair_completion,
    AgentWorkspaceRepairCompletionCheck,
};
use crate::application::GitService;
use crate::domain::entities::{AgentConversationWorkspacePublicationEvent, ChatConversationId};

#[derive(Debug, serde::Deserialize)]
pub struct CompleteAgentWorkspaceRepairRequest {
    pub repair_commit_sha: String,
    pub resolved_base_ref: String,
    pub resolved_base_commit: String,
    pub summary: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CompleteAgentWorkspaceRepairResponse {
    pub success: bool,
    pub message: String,
    pub new_status: String,
    pub base_commit: String,
    pub repair_commit_sha: String,
}

/// POST /api/agent-workspaces/{conversation_id}/complete-repair
///
/// Called by the dedicated agent workspace repair agent after it has resolved a
/// publish/update failure and committed the repair.
pub async fn complete_agent_workspace_repair(
    State(state): State<HttpServerState>,
    Path(conversation_id): Path<String>,
    Json(req): Json<CompleteAgentWorkspaceRepairRequest>,
) -> Result<Json<CompleteAgentWorkspaceRepairResponse>, JsonError> {
    if !is_valid_git_sha(&req.repair_commit_sha) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "repair_commit_sha must be a full 40-character SHA (use `git rev-parse HEAD`)",
            None,
        ));
    }
    if !is_valid_git_sha(&req.resolved_base_commit) {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "resolved_base_commit must be a full 40-character SHA",
            None,
        ));
    }

    let conversation_id = ChatConversationId::from_string(conversation_id);
    let workspace = state
        .app_state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation_id)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Agent workspace not found", None))?;

    let project = state
        .app_state
        .project_repo
        .get_by_id(&workspace.project_id)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Project not found", None))?;

    let workspace_path = resolve_valid_agent_conversation_workspace_path(&project, &workspace)
        .await
        .map_err(|error| json_error(StatusCode::BAD_REQUEST, error.to_string(), None))?;

    let freshness = inspect_publish_branch_freshness_for_source(
        &workspace_path,
        &workspace.base_ref,
        &workspace.branch_name,
        workspace.base_commit.as_deref(),
    )
    .await
    .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;

    let workspace_head_sha = GitService::get_head_sha(&workspace_path)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;
    let has_uncommitted_changes = GitService::has_uncommitted_changes(&workspace_path)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;
    let has_conflict_markers = GitService::has_conflict_markers(&workspace_path)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;

    verify_agent_workspace_repair_completion(AgentWorkspaceRepairCompletionCheck {
        freshness_status: &freshness,
        workspace_base_ref: &workspace.base_ref,
        resolved_base_ref: &req.resolved_base_ref,
        resolved_base_commit: &req.resolved_base_commit,
        repair_commit_sha: &req.repair_commit_sha,
        workspace_head_sha: &workspace_head_sha,
        has_uncommitted_changes,
        is_merge_in_progress: GitService::is_merge_in_progress(&workspace_path),
        is_rebase_in_progress: GitService::is_rebase_in_progress(&workspace_path),
        has_conflict_markers,
    })
    .map_err(|error| json_error(StatusCode::CONFLICT, error, None))?;

    let mut updated_workspace = workspace.clone();
    updated_workspace.base_commit = Some(freshness.target_base_commit.clone());
    updated_workspace.publication_push_status = Some("refreshed".to_string());
    state
        .app_state
        .agent_conversation_workspace_repo
        .create_or_update(updated_workspace)
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;
    state
        .app_state
        .agent_conversation_workspace_repo
        .append_publication_event(AgentConversationWorkspacePublicationEvent::new(
            conversation_id,
            "repair_completed",
            "succeeded",
            req.summary.clone(),
            Some("agent_fixable".to_string()),
        ))
        .await
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string(), None))?;

    Ok(Json(CompleteAgentWorkspaceRepairResponse {
        success: true,
        message: "Agent workspace repair verified".to_string(),
        new_status: "refreshed".to_string(),
        base_commit: freshness.target_base_commit,
        repair_commit_sha: req.repair_commit_sha,
    }))
}
