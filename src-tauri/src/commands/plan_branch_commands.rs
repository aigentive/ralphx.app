// Plan branch commands for feature branch management
// Thin layer bridging frontend to plan branch repository and git operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::application::git_service::GitService;
use crate::application::AppState;
use crate::domain::entities::{
    ArtifactId, IdeationSessionId, InternalStatus, PlanBranch, PlanBranchStatus, ProjectId, Task,
    TaskId,
};

/// Response for plan branch queries
#[derive(Debug, Serialize)]
pub struct PlanBranchResponse {
    pub id: String,
    pub plan_artifact_id: String,
    pub session_id: String,
    pub project_id: String,
    pub branch_name: String,
    pub source_branch: String,
    pub status: String,
    pub merge_task_id: Option<String>,
    pub created_at: String,
    pub merged_at: Option<String>,
}

impl From<PlanBranch> for PlanBranchResponse {
    fn from(pb: PlanBranch) -> Self {
        Self {
            id: pb.id.as_str().to_string(),
            plan_artifact_id: pb.plan_artifact_id.as_str().to_string(),
            session_id: pb.session_id.as_str().to_string(),
            project_id: pb.project_id.as_str().to_string(),
            branch_name: pb.branch_name,
            source_branch: pb.source_branch,
            status: pb.status.to_db_string().to_string(),
            merge_task_id: pb.merge_task_id.map(|id| id.as_str().to_string()),
            created_at: pb.created_at.to_rfc3339(),
            merged_at: pb.merged_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Input for enable_feature_branch command
#[derive(Debug, Deserialize)]
pub struct EnableFeatureBranchInput {
    pub plan_artifact_id: String,
    pub session_id: String,
    pub project_id: String,
}

// ============================================================================
// Query Commands
// ============================================================================

/// Get plan branch by plan artifact ID
///
/// Returns the plan branch for a given plan artifact, or null if none exists.
#[tauri::command]
pub async fn get_plan_branch(
    plan_artifact_id: String,
    state: State<'_, AppState>,
) -> Result<Option<PlanBranchResponse>, String> {
    let artifact_id = ArtifactId::from_string(plan_artifact_id);

    state
        .plan_branch_repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .map(|opt| opt.map(PlanBranchResponse::from))
        .map_err(|e| e.to_string())
}

/// Get all plan branches for a project
///
/// Returns all feature branches associated with a project.
#[tauri::command]
pub async fn get_project_plan_branches(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PlanBranchResponse>, String> {
    let project_id = ProjectId::from_string(project_id);

    state
        .plan_branch_repo
        .get_by_project_id(&project_id)
        .await
        .map(|branches| branches.into_iter().map(PlanBranchResponse::from).collect())
        .map_err(|e| e.to_string())
}

// ============================================================================
// Mutation Commands
// ============================================================================

/// Enable feature branch for a plan (mid-plan conversion)
///
/// Creates a git feature branch, DB record, and merge task with dependencies
/// on all unmerged plan tasks. Used when enabling feature branches after
/// some tasks may have already been created/merged.
#[tauri::command]
pub async fn enable_feature_branch(
    input: EnableFeatureBranchInput,
    state: State<'_, AppState>,
) -> Result<PlanBranchResponse, String> {
    let plan_artifact_id = ArtifactId::from_string(input.plan_artifact_id);
    let session_id = IdeationSessionId::from_string(input.session_id);
    let project_id = ProjectId::from_string(input.project_id);

    // Check if a feature branch already exists for this plan
    let existing = state
        .plan_branch_repo
        .get_by_plan_artifact_id(&plan_artifact_id)
        .await
        .map_err(|e| e.to_string())?;
    if existing.is_some() {
        return Err("Feature branch already exists for this plan".to_string());
    }

    // Get the project for base branch and working directory
    let project = state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))?;

    let base_branch = project
        .base_branch
        .as_deref()
        .unwrap_or("main")
        .to_string();
    let repo_path = PathBuf::from(&project.working_directory);

    // Generate branch name: ralphx/{project-slug}/plan-{short-artifact-id}
    let project_slug = slug_from_name(&project.name);
    let short_id = &plan_artifact_id.as_str()[..8.min(plan_artifact_id.as_str().len())];
    let branch_name = format!("ralphx/{}/plan-{}", project_slug, short_id);

    // Create git feature branch from base branch
    GitService::create_feature_branch(&repo_path, &branch_name, &base_branch)
        .map_err(|e| format!("Failed to create feature branch: {}", e))?;

    // Insert plan_branches DB record
    let plan_branch = PlanBranch::new(
        plan_artifact_id.clone(),
        session_id.clone(),
        project_id.clone(),
        branch_name,
        base_branch.clone(),
    );
    let created_branch = state
        .plan_branch_repo
        .create(plan_branch)
        .await
        .map_err(|e| format!("Failed to create plan branch record: {}", e))?;

    // Find all unmerged plan tasks for this plan artifact.
    // Also find tasks linked via session proposals whose plan_artifact_id is NULL
    // (created before the session_id fallback fix), and backfill their plan_artifact_id.
    let all_tasks = state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|e| e.to_string())?;

    // Collect task IDs from session proposals (for tasks with NULL plan_artifact_id)
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;
    let session_task_ids: std::collections::HashSet<TaskId> = proposals
        .iter()
        .filter_map(|p| p.created_task_id.clone())
        .collect();

    // Backfill plan_artifact_id on tasks that were created from session proposals but lack it
    for task in &all_tasks {
        if task.plan_artifact_id.is_none() && session_task_ids.contains(&task.id) {
            let mut task_to_update = task.clone();
            task_to_update.plan_artifact_id = Some(plan_artifact_id.clone());
            state
                .task_repo
                .update(&task_to_update)
                .await
                .map_err(|e| format!("Failed to backfill plan_artifact_id: {}", e))?;
        }
    }

    let unmerged_plan_tasks: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| {
            let matches_plan = t.plan_artifact_id.as_ref() == Some(&plan_artifact_id)
                || (t.plan_artifact_id.is_none() && session_task_ids.contains(&t.id));
            matches_plan
                && t.internal_status != InternalStatus::Merged
                && t.internal_status != InternalStatus::Approved
        })
        .collect();

    // Create merge task
    let plan_title = format!("Merge plan into {}", base_branch);
    let mut merge_task =
        Task::new_with_category(project_id.clone(), plan_title, "plan_merge".to_string());
    merge_task.description = Some(format!(
        "Auto-created merge task: merges feature branch into {}",
        base_branch
    ));
    merge_task.plan_artifact_id = Some(plan_artifact_id);
    merge_task.internal_status = InternalStatus::Blocked;
    merge_task.blocked_reason = Some("Waiting for all plan tasks to complete".to_string());

    let created_merge_task = state
        .task_repo
        .create(merge_task)
        .await
        .map_err(|e| format!("Failed to create merge task: {}", e))?;

    // Add blockedBy dependencies: merge task blocked by all unmerged plan tasks
    for plan_task in &unmerged_plan_tasks {
        state
            .task_dependency_repo
            .add_dependency(&created_merge_task.id, &plan_task.id)
            .await
            .map_err(|e| format!("Failed to add dependency: {}", e))?;
    }

    // Set merge_task_id on the plan branch record
    state
        .plan_branch_repo
        .set_merge_task_id(&created_branch.id, &created_merge_task.id)
        .await
        .map_err(|e| format!("Failed to set merge task ID: {}", e))?;

    // Re-fetch the plan branch to get updated data
    let updated_branch = state
        .plan_branch_repo
        .get_by_plan_artifact_id(&created_branch.plan_artifact_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Plan branch not found after creation".to_string())?;

    Ok(PlanBranchResponse::from(updated_branch))
}

/// Disable feature branch for a plan
///
/// Only allowed if no tasks have been merged to the feature branch yet.
/// Removes the git branch, DB record, and merge task.
#[tauri::command]
pub async fn disable_feature_branch(
    plan_artifact_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let artifact_id = ArtifactId::from_string(plan_artifact_id);

    // Get the plan branch
    let plan_branch = state
        .plan_branch_repo
        .get_by_plan_artifact_id(&artifact_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No feature branch found for this plan".to_string())?;

    // Only allow disabling active branches
    if plan_branch.status != PlanBranchStatus::Active {
        return Err(format!(
            "Cannot disable feature branch with status: {}",
            plan_branch.status
        ));
    }

    // Check if any tasks have been merged to the feature branch
    let project = state
        .project_repo
        .get_by_id(&plan_branch.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    let all_tasks = state
        .task_repo
        .get_by_project(&plan_branch.project_id)
        .await
        .map_err(|e| e.to_string())?;
    let has_merged_tasks = all_tasks.iter().any(|t| {
        t.plan_artifact_id.as_ref() == Some(&artifact_id)
            && t.internal_status == InternalStatus::Merged
    });

    if has_merged_tasks {
        return Err(
            "Cannot disable feature branch: tasks have already been merged to it".to_string(),
        );
    }

    // Delete the merge task if it exists
    if let Some(merge_task_id) = &plan_branch.merge_task_id {
        state
            .task_repo
            .delete(merge_task_id)
            .await
            .map_err(|e| format!("Failed to delete merge task: {}", e))?;
    }

    // Delete the git branch
    let repo_path = PathBuf::from(&project.working_directory);
    if let Err(e) = GitService::delete_feature_branch(&repo_path, &plan_branch.branch_name) {
        tracing::warn!(
            "Failed to delete git branch {}: {} (continuing with DB cleanup)",
            plan_branch.branch_name,
            e
        );
    }

    // Update plan branch status to Abandoned
    state
        .plan_branch_repo
        .update_status(&plan_branch.id, PlanBranchStatus::Abandoned)
        .await
        .map_err(|e| format!("Failed to update plan branch status: {}", e))?;

    Ok(())
}

/// Update project-level feature branch setting
///
/// Enables or disables feature branches as the default for new plans.
#[tauri::command]
pub async fn update_project_feature_branch_setting(
    project_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project_id = ProjectId::from_string(project_id);

    let mut project = state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))?;

    project.use_feature_branches = enabled;
    project.touch();

    state
        .project_repo
        .update(&project)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Helpers
// ============================================================================

/// Generate a URL-safe slug from a project name
pub fn slug_from_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_name_simple() {
        assert_eq!(slug_from_name("My Project"), "my-project");
    }

    #[test]
    fn slug_from_name_special_chars() {
        assert_eq!(slug_from_name("My App (v2.0)"), "my-app-v2-0");
    }

    #[test]
    fn slug_from_name_collapses_consecutive_hyphens() {
        assert_eq!(slug_from_name("foo---bar"), "foo-bar");
    }

    #[test]
    fn slug_from_name_trims_leading_trailing() {
        assert_eq!(slug_from_name(" Hello World "), "hello-world");
    }

    #[test]
    fn plan_branch_response_from_entity() {
        let pb = PlanBranch::new(
            ArtifactId::from_string("art-1"),
            IdeationSessionId::from_string("sess-1"),
            ProjectId::from_string("proj-1".to_string()),
            "ralphx/my-app/plan-a1b2c3".to_string(),
            "main".to_string(),
        );

        let response = PlanBranchResponse::from(pb);
        assert_eq!(response.plan_artifact_id, "art-1");
        assert_eq!(response.branch_name, "ralphx/my-app/plan-a1b2c3");
        assert_eq!(response.source_branch, "main");
        assert_eq!(response.status, "active");
        assert!(response.merge_task_id.is_none());
        assert!(response.merged_at.is_none());
    }
}
