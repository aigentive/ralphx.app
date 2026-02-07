// Transition handler - orchestrates side effects for state transitions
// This module wraps the state machine and handles entry/exit actions,
// especially for QA-related transitions.

use super::events::TaskEvent;
use super::machine::{Response, State, TaskStateMachine};
use crate::application::GitService;
use crate::domain::entities::{GitMode, ProjectId, TaskId};

mod side_effects;
#[cfg(test)]
mod tests;

// Re-export shared merge completion logic for use by HTTP handlers and auto-completion
pub use side_effects::complete_merge_internal;
pub use side_effects::resolve_merge_branches;

/// Result of handling a transition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition completed successfully
    Success(State),
    /// Event was not handled in current state
    NotHandled,
    /// Auto-transition triggered (e.g., ExecutionDone -> QaRefining)
    AutoTransition(State),
}

impl TransitionResult {
    /// Get the final state if transition was successful
    pub fn state(&self) -> Option<&State> {
        match self {
            TransitionResult::Success(s) | TransitionResult::AutoTransition(s) => Some(s),
            TransitionResult::NotHandled => None,
        }
    }

    /// Check if the transition resulted in a new state
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            TransitionResult::Success(_) | TransitionResult::AutoTransition(_)
        )
    }
}

/// Handler for state transitions with side effects
pub struct TransitionHandler<'a> {
    machine: &'a mut TaskStateMachine,
}

impl<'a> TransitionHandler<'a> {
    /// Create a new transition handler wrapping a state machine
    pub fn new(machine: &'a mut TaskStateMachine) -> Self {
        Self { machine }
    }

    /// Handle a state transition with side effects
    ///
    /// This method:
    /// 1. Dispatches the event to the state machine
    /// 2. Executes entry actions for the new state (if applicable)
    /// 3. Handles auto-transitions (e.g., ExecutionDone -> QaRefining when QA enabled)
    pub async fn handle_transition(
        &mut self,
        current_state: &State,
        event: &TaskEvent,
    ) -> TransitionResult {
        // Dispatch event to state machine
        let response = self.machine.dispatch(current_state, event);

        match response {
            Response::Transition(new_state) => {
                // Execute on-exit action for old state
                self.on_exit(current_state, &new_state).await;

                // Execute on-enter action for new state
                if let Err(e) = self.on_enter(&new_state).await {
                    tracing::error!(error = %e, "on_enter failed for state {:?}", new_state);
                    // Note: We still return Success as the transition happened,
                    // but side effects may not have completed
                }

                // Check for auto-transitions
                if let Some(auto_state) = self.check_auto_transition(&new_state) {
                    // Execute on-exit for intermediate state
                    self.on_exit(&new_state, &auto_state).await;
                    // Execute on-enter for final state
                    if let Err(e) = self.on_enter(&auto_state).await {
                        tracing::error!(error = %e, "on_enter failed for auto-transition state {:?}", auto_state);
                    }
                    return TransitionResult::AutoTransition(auto_state);
                }

                TransitionResult::Success(new_state)
            }
            Response::Handled => TransitionResult::Success(current_state.clone()),
            Response::NotHandled => TransitionResult::NotHandled,
        }
    }

    /// Execute on-exit action for a state
    ///
    /// This method is public to allow `TaskTransitionService` to trigger exit actions
    /// for direct status changes (e.g., stop command) without going through the
    /// full event-based transition flow. This ensures running count is properly
    /// decremented when tasks exit agent-active states.
    pub async fn on_exit(&self, from: &State, _to: &State) {
        // Decrement running count for agent-active states
        // This ensures ExecutionState tracks concurrency accurately
        match from {
            State::Executing | State::QaRefining | State::QaTesting | State::Reviewing | State::ReExecuting | State::Merging => {
                if let Some(ref exec) = self.machine.context.services.execution_state {
                    exec.decrement_running();
                    tracing::debug!(
                        task_id = %self.machine.context.task_id,
                        from_state = ?from,
                        new_count = exec.running_count(),
                        "Decremented running count on state exit"
                    );

                    // Emit real-time status update event to frontend
                    if let Some(ref handle) = self.machine.context.services.app_handle {
                        exec.emit_status_changed(handle, "task_completed");
                    }

                    // Try to schedule next Ready task now that a slot is free
                    self.try_schedule_ready_tasks().await;
                }
            }
            _ => {}
        }

        // State-specific exit actions
        match from {
            State::Executing | State::ReExecuting => {
                // Auto-commit on execution completion (Phase 66 - Task 7)
                // Commit any uncommitted changes with message: {prefix}{task_title}
                self.auto_commit_on_execution_done().await;
            }
            State::QaTesting => {
                // Stop QA tester if transitioning away
            }
            State::Reviewing => {
                // Log review duration (could add timing metrics here)
                // For now, just emit an event that review exited
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:state_exited", &self.machine.context.task_id)
                    .await;
            }
            _ => {}
        }
    }

    /// Check for auto-transitions from the given state
    pub fn check_auto_transition(&self, state: &State) -> Option<State> {
        match state {
            State::QaPassed => {
                // Auto-transition to PendingReview
                Some(State::PendingReview)
            }
            State::RevisionNeeded => {
                // Auto-transition to ReExecuting (revision work)
                Some(State::ReExecuting)
            }
            State::PendingReview => {
                // Auto-transition to Reviewing (spawn reviewer)
                Some(State::Reviewing)
            }
            State::Approved => {
                // Auto-transition to PendingMerge (Phase 66 - merge workflow)
                // NOTE: PendingMerge does NOT auto-transition - side effect determines next state
                Some(State::PendingMerge)
            }
            _ => None,
        }
    }

    /// Try to schedule Ready tasks if execution slots are available.
    ///
    /// This method delegates to the TaskScheduler service if available.
    /// Called from:
    /// - on_exit() when exiting agent-active states (slot freed)
    /// - on_enter(Ready) when a task becomes Ready
    ///
    /// The scheduler will check capacity and start the oldest Ready task
    /// across all projects if a slot is available.
    pub async fn try_schedule_ready_tasks(&self) {
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            tracing::debug!(
                task_id = %self.machine.context.task_id,
                "Triggering ready task scheduling"
            );
            scheduler.try_schedule_ready_tasks().await;
        }
    }

    /// Auto-commit on execution completion (Phase 66 - Task 7)
    ///
    /// When a task exits Executing or ReExecuting state, commit any uncommitted
    /// changes with message format: {prefix}{task_title}
    ///
    /// Default prefix: "feat: "
    /// TODO: Make configurable via ExecutionSettings when that infrastructure exists
    async fn auto_commit_on_execution_done(&self) {
        let task_id_str = &self.machine.context.task_id;
        let project_id_str = &self.machine.context.project_id;

        // Only proceed if task_repo and project_repo are available
        let (Some(ref task_repo), Some(ref project_repo)) = (
            &self.machine.context.services.task_repo,
            &self.machine.context.services.project_repo,
        ) else {
            tracing::debug!(
                task_id = task_id_str,
                "Skipping auto-commit: repos not available"
            );
            return;
        };

        let task_id = TaskId::from_string(task_id_str.clone());
        let project_id = ProjectId::from_string(project_id_str.clone());

        // Fetch task and project
        let task_result = task_repo.get_by_id(&task_id).await;
        let project_result = project_repo.get_by_id(&project_id).await;

        let (Ok(Some(task)), Ok(Some(project))) = (task_result, project_result) else {
            tracing::warn!(
                task_id = task_id_str,
                "Skipping auto-commit: failed to fetch task or project"
            );
            return;
        };

        // Resolve working directory based on git mode
        let working_path = resolve_working_directory(&task, &project);

        // Check for uncommitted changes
        match GitService::has_uncommitted_changes(&working_path) {
            Ok(true) => {
                // Build commit message: {prefix}{task_title}
                // Default prefix: "feat: " (configurable in future)
                let prefix = "feat: ";
                let message = format!("{}{}", prefix, task.title);

                match GitService::commit_all(&working_path, &message) {
                    Ok(Some(sha)) => {
                        tracing::info!(
                            task_id = task_id_str,
                            commit_sha = %sha,
                            message = %message,
                            "Auto-committed changes on execution completion"
                        );
                    }
                    Ok(None) => {
                        tracing::debug!(
                            task_id = task_id_str,
                            "Auto-commit: no staged changes to commit"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "Auto-commit failed (non-fatal)"
                        );
                    }
                }
            }
            Ok(false) => {
                tracing::debug!(
                    task_id = task_id_str,
                    "No uncommitted changes to auto-commit"
                );
            }
            Err(e) => {
                tracing::warn!(
                    task_id = task_id_str,
                    error = %e,
                    "Failed to check uncommitted changes (non-fatal)"
                );
            }
        }
    }
}

/// Resolve the working directory for a task based on git mode.
///
/// - Local mode: Always returns project's working directory (branch switching)
/// - Worktree mode: Returns task's worktree path if available, else project's working directory
fn resolve_working_directory(
    task: &crate::domain::entities::Task,
    project: &crate::domain::entities::Project,
) -> std::path::PathBuf {
    match project.git_mode {
        GitMode::Local => {
            // Local mode: always use main repo (branch switches handle isolation)
            std::path::PathBuf::from(&project.working_directory)
        }
        GitMode::Worktree => {
            // Worktree mode: use task's worktree if exists
            task.worktree_path
                .as_ref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from(&project.working_directory))
        }
    }
}
