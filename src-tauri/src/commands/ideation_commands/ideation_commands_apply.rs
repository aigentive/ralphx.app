// Proposal-to-task conversion and task dependency commands

use std::collections::{HashMap, HashSet};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{
    IdeationSessionId, IdeationSessionStatus, InternalStatus, Task,
    TaskId, TaskProposal, TaskProposalId,
};

use super::ideation_commands_types::{ApplyProposalsInput, ApplyProposalsResultResponse};

// ============================================================================
// Apply and Task Dependency Commands
// ============================================================================

/// Apply selected proposals to the Kanban board as tasks
#[tauri::command]
pub async fn apply_proposals_to_kanban(
    input: ApplyProposalsInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ApplyProposalsResultResponse, String> {
    use crate::commands::emit_queue_changed;

    let session_id = IdeationSessionId::from_string(input.session_id);

    // Status will be determined automatically based on dependencies:
    // - Tasks with no blockers → Ready
    // - Tasks with blockers → Blocked
    // The target_column field is kept for backwards compatibility but ignored when "auto"
    let use_auto_status = input.target_column.to_lowercase() == "auto";

    // Get the session to know the project_id
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    if session.status != IdeationSessionStatus::Active {
        return Err("Cannot apply proposals from an inactive session".to_string());
    }

    let proposal_ids: HashSet<TaskProposalId> = input
        .proposal_ids
        .into_iter()
        .map(TaskProposalId::from_string)
        .collect();

    // Validate that all proposals exist and belong to this session
    let all_proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    let proposals_to_apply: Vec<TaskProposal> = all_proposals
        .into_iter()
        .filter(|p| proposal_ids.contains(&p.id))
        .collect();

    if proposals_to_apply.len() != proposal_ids.len() {
        return Err("Some proposals not found in session".to_string());
    }

    // Create tasks and track dependencies
    let mut created_tasks = Vec::new();
    let mut proposal_to_task: HashMap<TaskProposalId, TaskId> = HashMap::new();
    let mut warnings = Vec::new();

    for proposal in &proposals_to_apply {
        // Create task from proposal
        let mut task = Task::new(session.project_id.clone(), proposal.title.clone());
        task.description = proposal.description.clone();
        task.category = proposal.category.to_string();
        // Initial status - will be updated after dependencies are created
        task.internal_status = InternalStatus::Backlog;

        // Set priority based on user override or suggested (use priority score as i32)
        if proposal.user_priority.is_some() {
            task.priority = proposal.priority_score; // Use calculated score
        } else {
            task.priority = proposal.priority_score;
        }

        let created_task = state
            .task_repo
            .create(task)
            .await
            .map_err(|e| e.to_string())?;

        // Import steps from proposal if they exist
        if let Some(steps_json) = &proposal.steps {
            if let Ok(step_titles) = serde_json::from_str::<Vec<String>>(steps_json) {
                if !step_titles.is_empty() {
                    let task_steps: Vec<crate::domain::entities::TaskStep> = step_titles
                        .into_iter()
                        .enumerate()
                        .map(|(idx, title)| {
                            crate::domain::entities::TaskStep::new(
                                created_task.id.clone(),
                                title,
                                idx as i32,
                                "proposal".to_string(),
                            )
                        })
                        .collect();

                    // Use bulk_create to insert all steps
                    let _ = state
                        .task_step_repo
                        .bulk_create(task_steps)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        proposal_to_task.insert(proposal.id.clone(), created_task.id.clone());
        created_tasks.push(created_task);
    }

    // Create task dependencies if requested
    let mut dependencies_created = 0;
    if input.preserve_dependencies {
        for proposal in &proposals_to_apply {
            let deps = state
                .proposal_dependency_repo
                .get_dependencies(&proposal.id)
                .await
                .map_err(|e| e.to_string())?;

            for dep_proposal_id in deps {
                if let (Some(task_id), Some(dep_task_id)) = (
                    proposal_to_task.get(&proposal.id),
                    proposal_to_task.get(&dep_proposal_id),
                ) {
                    state
                        .task_dependency_repo
                        .add_dependency(task_id, dep_task_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    dependencies_created += 1;
                } else {
                    warnings.push(format!(
                        "Dependency from {} to {} not preserved (not in selection)",
                        proposal.id, dep_proposal_id
                    ));
                }
            }
        }
    }

    // Update proposal statuses and link to created tasks
    for proposal in &proposals_to_apply {
        if let Some(task_id) = proposal_to_task.get(&proposal.id) {
            state
                .task_proposal_repo
                .set_created_task_id(&proposal.id, task_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Auto-status: Set task status based on dependencies
    // - Tasks with no blockers → Ready
    // - Tasks with blockers → Blocked (with blocked_reason)
    let mut any_ready_tasks = false;
    if use_auto_status {
        // Build a map of task_id -> task title for blocker names
        let task_titles: HashMap<TaskId, String> = created_tasks
            .iter()
            .map(|t| (t.id.clone(), t.title.clone()))
            .collect();

        for task in &created_tasks {
            let blockers = state
                .task_dependency_repo
                .get_blockers(&task.id)
                .await
                .map_err(|e| e.to_string())?;

            // Fetch the task to update it
            let mut task_to_update = state
                .task_repo
                .get_by_id(&task.id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Task {} not found after creation", task.id))?;

            if blockers.is_empty() {
                // No blockers - set to Ready
                task_to_update.internal_status = InternalStatus::Ready;
                task_to_update.blocked_reason = None;
                any_ready_tasks = true;
            } else {
                // Has blockers - set to Blocked with reason
                let blocker_names: Vec<String> = blockers
                    .iter()
                    .filter_map(|blocker_id| task_titles.get(blocker_id))
                    .cloned()
                    .collect();
                let blocked_reason = format!("Waiting for: {}", blocker_names.join(", "));

                task_to_update.internal_status = InternalStatus::Blocked;
                task_to_update.blocked_reason = Some(blocked_reason);
            }

            state
                .task_repo
                .update(&task_to_update)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Check if all proposals in session are now applied
    let remaining = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p.created_task_id.is_none())
        .count();

    let session_converted = remaining == 0;
    if session_converted {
        state
            .ideation_session_repo
            .update_status(&session_id, IdeationSessionStatus::Accepted)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Emit queue_changed if any tasks were set to Ready status
    if any_ready_tasks {
        emit_queue_changed(&state, &session.project_id, &app).await;
    }

    Ok(ApplyProposalsResultResponse {
        created_task_ids: created_tasks
            .into_iter()
            .map(|t| t.id.as_str().to_string())
            .collect(),
        dependencies_created,
        warnings,
        session_converted,
    })
}

/// Get blockers for a task (tasks it depends on)
#[tauri::command]
pub async fn get_task_blockers(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .task_dependency_repo
        .get_blockers(&task_id)
        .await
        .map(|blockers| {
            blockers
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get tasks blocked by a task (tasks that depend on this one)
#[tauri::command]
pub async fn get_blocked_tasks(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let task_id = TaskId::from_string(task_id);

    state
        .task_dependency_repo
        .get_blocked_by(&task_id)
        .await
        .map(|blocked| {
            blocked
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
}
