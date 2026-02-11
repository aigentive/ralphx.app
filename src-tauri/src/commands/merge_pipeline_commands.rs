// Merge Pipeline Query Commands
//
// Provides visibility into the merge pipeline: active merges, waiting merges,
// and tasks needing attention (merge_conflict, merge_incomplete).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::application::AppState;
use crate::commands::execution_commands::ActiveProjectState;
use crate::domain::entities::ProjectId;
use crate::domain::entities::{InternalStatus, Task};
use crate::domain::state_machine::transition_handler::{
    has_merge_deferred_metadata, resolve_merge_branches,
};

/// A task in the merge pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePipelineTask {
    /// Task ID
    pub task_id: String,
    /// Task title
    pub title: String,
    /// Current internal status
    pub internal_status: String,
    /// Source branch (task branch)
    pub source_branch: String,
    /// Target branch (resolved merge target)
    pub target_branch: String,
    /// Whether this merge is deferred (waiting for another merge)
    pub is_deferred: bool,
    /// Blocking branch name (for deferred merges)
    pub blocking_branch: Option<String>,
    /// Conflict file paths (for merge_conflict tasks)
    pub conflict_files: Option<Vec<String>>,
    /// Error context (for merge_incomplete tasks)
    pub error_context: Option<String>,
}

/// Response for get_merge_pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePipelineResponse {
    /// Tasks currently merging (status = merging)
    pub active: Vec<MergePipelineTask>,
    /// Tasks waiting to merge (status = pending_merge)
    pub waiting: Vec<MergePipelineTask>,
    /// Tasks needing attention (status = merge_conflict | merge_incomplete)
    pub needs_attention: Vec<MergePipelineTask>,
}

/// Extract merge metadata (conflict files, error context) from task metadata JSON
fn extract_merge_metadata(task: &Task) -> (Option<Vec<String>>, Option<String>) {
    use crate::domain::state_machine::transition_handler::parse_metadata;

    let meta = parse_metadata(task);
    if let Some(meta) = meta {
        let conflict_files = meta
            .get("conflict_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        let error_context = meta.get("error").and_then(|v| v.as_str()).map(String::from);

        (conflict_files, error_context)
    } else {
        (None, None)
    }
}

/// Get the merge pipeline for all projects
///
/// Returns tasks in merge-related states grouped into:
/// - active: tasks currently merging (status = merging)
/// - waiting: tasks waiting to merge (status = pending_merge)
/// - needs_attention: tasks with conflicts or errors (status = merge_conflict | merge_incomplete)
#[tauri::command]
pub async fn get_merge_pipeline(
    project_id: Option<String>,
    active_project_state: State<'_, Arc<ActiveProjectState>>,
    state: State<'_, AppState>,
) -> Result<MergePipelineResponse, String> {
    let effective_project_id = match project_id {
        Some(id) => Some(ProjectId::from_string(id)),
        None => active_project_state.get().await,
    };

    let mut active = Vec::new();
    let mut waiting = Vec::new();
    let mut needs_attention = Vec::new();

    // Merge-related statuses
    let merge_statuses = [
        InternalStatus::Merging,
        InternalStatus::PendingMerge,
        InternalStatus::MergeConflict,
        InternalStatus::MergeIncomplete,
    ];

    let projects = if let Some(pid) = &effective_project_id {
        match state
            .project_repo
            .get_by_id(pid)
            .await
            .map_err(|e| e.to_string())?
        {
            Some(project) => vec![project],
            None => Vec::new(),
        }
    } else {
        state
            .project_repo
            .get_all()
            .await
            .map_err(|e| e.to_string())?
    };

    for project in &projects {
        let tasks = state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            // Filter by merge-related statuses
            if !merge_statuses.contains(&task.internal_status) {
                continue;
            }

            // Resolve merge branches
            let (source_branch, target_branch) =
                resolve_merge_branches(&task, &project, &Some(Arc::clone(&state.plan_branch_repo)))
                    .await;

            // Check if deferred
            let is_deferred = has_merge_deferred_metadata(&task);

            // Extract merge metadata
            let (conflict_files, error_context) = extract_merge_metadata(&task);

            // Determine blocking branch for deferred tasks
            let blocking_branch = if is_deferred {
                // Find the active merging task for this project to determine blocking branch
                let active_merges = state
                    .task_repo
                    .get_by_status(&task.project_id, InternalStatus::Merging)
                    .await
                    .map_err(|e| e.to_string())?;

                active_merges.first().and_then(|t| t.task_branch.clone())
            } else {
                None
            };

            let pipeline_task = MergePipelineTask {
                task_id: task.id.as_str().to_string(),
                title: task.title.clone(),
                internal_status: task.internal_status.as_str().to_string(),
                source_branch,
                target_branch,
                is_deferred,
                blocking_branch,
                conflict_files,
                error_context,
            };

            match task.internal_status {
                InternalStatus::Merging => active.push(pipeline_task),
                InternalStatus::PendingMerge => waiting.push(pipeline_task),
                InternalStatus::MergeConflict | InternalStatus::MergeIncomplete => {
                    needs_attention.push(pipeline_task)
                }
                _ => {}
            }
        }
    }

    Ok(MergePipelineResponse {
        active,
        waiting,
        needs_attention,
    })
}
