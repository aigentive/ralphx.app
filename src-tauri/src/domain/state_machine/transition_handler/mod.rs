// Transition handler - orchestrates side effects for state transitions
// This module wraps the state machine and handles entry/exit actions,
// especially for QA-related transitions.

use super::events::TaskEvent;
use super::machine::{Response, State, TaskStateMachine};

pub(crate) mod cleanup_helpers;
mod checkout_free_strategy;
mod commit_messages;
mod exit_actions;
mod merge_completion;
mod merge_coordination;
mod merge_helpers;
mod merge_orchestrator;
mod merge_outcome_handler;
mod merge_strategies;
mod merge_validation;
pub mod metadata_builder;
pub(crate) mod on_enter_states;
mod side_effects;
#[cfg(test)]
mod tests;

// -- Public re-exports --
pub use merge_completion::complete_merge_internal;
pub use merge_helpers::resolve_merge_branches;
pub use metadata_builder::{build_failed_metadata, build_trigger_origin_metadata, MetadataUpdate};

// -- Crate-visible re-exports (merge_helpers) --
pub(crate) use merge_helpers::{
    clear_main_merge_deferred_metadata, clear_merge_deferred_metadata, get_trigger_origin,
    has_branch_missing_metadata, has_main_merge_deferred_metadata, has_merge_deferred_metadata,
    is_main_merge_deferred_timed_out, is_merge_deferred_timed_out, parse_metadata,
    set_conflict_metadata, set_trigger_origin, DEFERRED_MERGE_TIMEOUT_SECONDS,
};

// -- Crate-visible re-exports (merge_validation) --
pub(crate) use merge_validation::{format_validation_error_metadata, run_validation_commands};

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
    pub fn state(&self) -> Option<&State> {
        match self {
            TransitionResult::Success(s) | TransitionResult::AutoTransition(s) => Some(s),
            TransitionResult::NotHandled => None,
        }
    }

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
    pub fn new(machine: &'a mut TaskStateMachine) -> Self {
        Self { machine }
    }

    /// Build an ExitContext snapshot from the current machine context.
    fn exit_context(&self) -> exit_actions::ExitContext {
        let ctx = &self.machine.context;
        exit_actions::ExitContext {
            task_id: ctx.task_id.clone(),
            project_id: ctx.project_id.clone(),
            task_repo: ctx.services.task_repo.clone(),
            project_repo: ctx.services.project_repo.clone(),
            task_scheduler: ctx.services.task_scheduler.clone(),
            execution_state: ctx.services.execution_state.clone(),
        }
    }

    /// Handle a state transition with side effects
    ///
    /// 1. Dispatches the event to the state machine
    /// 2. Executes entry actions for the new state (if applicable)
    /// 3. Handles auto-transitions (e.g., ExecutionDone -> QaRefining when QA enabled)
    pub async fn handle_transition(
        &mut self,
        current_state: &State,
        event: &TaskEvent,
    ) -> TransitionResult {
        let response = self.machine.dispatch(current_state, event);

        match response {
            Response::Transition(new_state) => {
                self.on_exit(current_state, &new_state).await;

                if let Err(e) = self.on_enter(&new_state).await {
                    self.emit_on_enter_error(&new_state, &e).await;
                    tracing::error!(error = %e, "on_enter failed for state {:?}", new_state);

                    if matches!(e, crate::error::AppError::ExecutionBlocked(_)) {
                        let error_msg = e.to_string();
                        tracing::warn!("ExecutionBlocked detected, dispatching ExecutionFailed to transition to Failed");
                        let failed_event = TaskEvent::ExecutionFailed { error: error_msg };
                        let failed_response = self.machine.dispatch(&new_state, &failed_event);
                        if let Response::Transition(failed_state) = failed_response {
                            self.on_exit(&new_state, &failed_state).await;
                            if let Err(e) = self.on_enter(&failed_state).await {
                                tracing::error!(error = %e, "on_enter failed for recovery state {:?}", failed_state);
                            }
                            return TransitionResult::Success(failed_state);
                        }
                    }
                }

                if let Some(mut auto_state) = self.check_auto_transition(&new_state) {
                    if matches!(new_state, State::RevisionNeeded)
                        && matches!(auto_state, State::ReExecuting)
                    {
                        let ctx = self.exit_context();
                        auto_state =
                            exit_actions::check_revision_cap_or_fail(&ctx, auto_state).await;
                    }

                    self.on_exit(&new_state, &auto_state).await;
                    if let Err(e) = self.on_enter(&auto_state).await {
                        self.emit_on_enter_error(&auto_state, &e).await;
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

    /// Emit a task:on_enter_error event for UI visibility.
    async fn emit_on_enter_error(&self, state: &State, error: &crate::error::AppError) {
        self.machine
            .context
            .services
            .event_emitter
            .emit_with_payload(
                "task:on_enter_error",
                &self.machine.context.task_id,
                &format!(r#"{{"state":"{:?}","error":"{}"}}"#, state, error),
            )
            .await;
    }

    /// Execute on-exit action for a state.
    ///
    /// Public so `TaskTransitionService` can trigger exit actions for direct status
    /// changes (e.g., stop command) without the full event-based transition flow.
    pub async fn on_exit(&self, from: &State, to: &State) {
        // Decrement running count for agent-active states
        match from {
            State::Executing
            | State::QaRefining
            | State::QaTesting
            | State::Reviewing
            | State::ReExecuting
            | State::Merging => {
                if let Some(ref exec) = self.machine.context.services.execution_state {
                    exec.decrement_running();
                    let new_count = exec.running_count();
                    tracing::debug!(
                        task_id = %self.machine.context.task_id,
                        from_state = ?from,
                        new_count = new_count,
                        "Decremented running count on state exit"
                    );

                    if let Some(ref handle) = self.machine.context.services.app_handle {
                        exec.emit_status_changed(handle, "task_completed");
                    }

                    if new_count == 0 {
                        tracing::info!(
                            task_id = %self.machine.context.task_id,
                            "All agents idle, triggering main merge retry"
                        );
                        self.try_retry_main_merges().await;
                    }

                    self.try_schedule_ready_tasks().await;
                }

                let ctx = self.exit_context();
                exit_actions::clear_trigger_origin_on_exit(&ctx).await;
            }
            _ => {}
        }

        // State-specific exit actions
        match from {
            State::Executing | State::ReExecuting => {
                let ctx = self.exit_context();
                exit_actions::auto_commit_on_execution_done(&ctx).await;
            }
            State::QaTesting => {}
            State::Reviewing => {
                self.machine
                    .context
                    .services
                    .event_emitter
                    .emit("review:state_exited", &self.machine.context.task_id)
                    .await;
            }
            State::PendingMerge | State::Merging => {
                let ctx = self.exit_context();
                exit_actions::spawn_deferred_merge_retry(&ctx, from, to);
            }
            _ => {}
        }
    }

    /// Check for auto-transitions from the given state
    pub fn check_auto_transition(&self, state: &State) -> Option<State> {
        match state {
            State::QaPassed => Some(State::PendingReview),
            State::RevisionNeeded => Some(State::ReExecuting),
            State::PendingReview => Some(State::Reviewing),
            State::Approved => Some(State::PendingMerge),
            _ => None,
        }
    }

    /// Try to schedule Ready tasks if execution slots are available.
    pub async fn try_schedule_ready_tasks(&self) {
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            tracing::debug!(
                task_id = %self.machine.context.task_id,
                "Triggering ready task scheduling"
            );
            scheduler.try_schedule_ready_tasks().await;
        }
    }

    /// Retry main-branch merges that were deferred because agents were running.
    pub async fn try_retry_main_merges(&self) {
        if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
            tracing::debug!(
                task_id = %self.machine.context.task_id,
                "Triggering main merge retry (all agents idle)"
            );
            scheduler.try_retry_main_merges().await;
        }
    }
}
