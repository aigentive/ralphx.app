// Transition handler - orchestrates side effects for state transitions
// This module wraps the state machine and handles entry/exit actions,
// especially for QA-related transitions.

use super::events::TaskEvent;
use super::machine::{Response, State, TaskStateMachine};

mod side_effects;
#[cfg(test)]
mod tests;

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
                self.on_enter(&new_state).await;

                // Check for auto-transitions
                if let Some(auto_state) = self.check_auto_transition(&new_state) {
                    // Execute on-exit for intermediate state
                    self.on_exit(&new_state, &auto_state).await;
                    // Execute on-enter for final state
                    self.on_enter(&auto_state).await;
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
            State::Executing | State::QaRefining | State::QaTesting | State::Reviewing | State::ReExecuting => {
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
                }
            }
            _ => {}
        }

        // State-specific exit actions
        match from {
            State::Executing => {
                // Stop worker agent if it's still running
                // (This is a safety net - normally the agent completes naturally)
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
            _ => None,
        }
    }
}
