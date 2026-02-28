// TaskStateMachineRepository - SQLite-backed state machine integration
// Provides load/persist operations for task state machine
//
// NOTE: This file is intentionally exempt from the DbConnection (spawn_blocking) migration.
// Reasons:
// 1. All public methods are SYNCHRONOUS (not async) — DbConnection::run() is async-only.
// 2. Methods use SQLite transactions (unchecked_transaction) that span multiple operations;
//    these cannot be split across separate spawn_blocking calls.
// 3. Tests directly access `repo.conn.lock().unwrap()` to set up state, which requires
//    std::sync::Mutex semantics (not tokio::sync::Mutex used by DbConnection).
// 4. This repository is only used in tests (not called from any production async context),
//    so there is no risk of blocking the tokio timer driver.

use crate::domain::entities::TaskId;
use crate::domain::state_machine::{
    state_has_data, State, StateData, TaskContext, TaskEvent, TaskStateMachine,
};
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::sync::Mutex;

/// Repository for task state machine persistence.
///
/// This repository handles:
/// - Loading state machine state from SQLite
/// - Processing events and persisting new state
/// - Managing state-local data (QaFailed, Failed)
/// - Atomic transitions with transactions
pub struct TaskStateMachineRepository {
    conn: Mutex<Connection>,
}

impl TaskStateMachineRepository {
    /// Creates a new repository with the given connection.
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Loads the current state for a task.
    ///
    /// Returns the State enum, rehydrating state-local data if present.
    pub fn load_state(&self, task_id: &TaskId) -> AppResult<State> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Get current internal_status from tasks table
        let status_str: String = conn
            .query_row(
                "SELECT internal_status FROM tasks WHERE id = ?1",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    AppError::TaskNotFound(task_id.as_str().to_string())
                } else {
                    AppError::Database(e.to_string())
                }
            })?;

        // Parse status to state (with default data for states that have it)
        let state: State = status_str
            .parse()
            .map_err(|_| AppError::Database(format!("Invalid status: {}", status_str)))?;

        // If state has data, try to load it from task_state_data
        if state_has_data(&state) {
            if let Some(state_data) = self.load_state_data_inner(&conn, task_id)? {
                return Ok(state_data.apply_to_state(state));
            }
        }

        Ok(state)
    }

    /// Loads state data for a task (if any).
    fn load_state_data_inner(
        &self,
        conn: &Connection,
        task_id: &TaskId,
    ) -> AppResult<Option<StateData>> {
        let result = conn.query_row(
            "SELECT state_type, data FROM task_state_data WHERE task_id = ?1",
            [task_id.as_str()],
            |row| {
                let state_type: String = row.get(0)?;
                let data: String = row.get(1)?;
                Ok(StateData::new(state_type, data))
            },
        );

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// Persists a state change for a task.
    ///
    /// Updates the tasks table internal_status and manages state-local data.
    pub fn persist_state(&self, task_id: &TaskId, state: &State) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;
        self.persist_state_inner(&conn, task_id, state)
    }

    /// Internal state persistence logic.
    fn persist_state_inner(
        &self,
        conn: &Connection,
        task_id: &TaskId,
        state: &State,
    ) -> AppResult<()> {
        // Update internal_status in tasks table
        let affected = conn
            .execute(
                "UPDATE tasks SET internal_status = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = ?2",
                [state.as_str(), task_id.as_str()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(AppError::TaskNotFound(task_id.as_str().to_string()));
        }

        // Handle state-local data
        if state_has_data(state) {
            // Save state data
            if let Some(state_data) = StateData::from_state(state) {
                conn.execute(
                    "INSERT OR REPLACE INTO task_state_data (task_id, state_type, data, updated_at)
                     VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
                    [task_id.as_str(), &state_data.state_type, &state_data.data],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            }
        } else {
            // Clean up any existing state data for states without data
            conn.execute(
                "DELETE FROM task_state_data WHERE task_id = ?1",
                [task_id.as_str()],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// Processes an event and persists the resulting state.
    ///
    /// Returns the new state after processing the event.
    /// Returns an error if:
    /// - The task is not found
    /// - The event is not valid in the current state
    /// - A database error occurs
    pub fn process_event(&self, task_id: &TaskId, event: &TaskEvent) -> AppResult<State> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Start transaction
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Load current state
        let current_state = self.load_state_from_conn(&tx, task_id)?;

        // Create state machine with minimal context for event processing
        let context = TaskContext::new_test(task_id.as_str(), "");
        let mut machine = TaskStateMachine::new(context);

        // Process event
        let response = machine.dispatch(&current_state, event);

        let new_state = match response {
            crate::domain::state_machine::Response::Transition(new_state) => new_state,
            crate::domain::state_machine::Response::NotHandled => {
                return Err(AppError::InvalidTransition {
                    from: current_state.as_str().to_string(),
                    to: format!("event {:?} not handled", event),
                });
            }
            crate::domain::state_machine::Response::Handled => {
                // State didn't change, just return current
                return Ok(current_state);
            }
        };

        // Persist new state
        self.persist_state_in_tx(&tx, task_id, &new_state)?;

        // Commit transaction
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(new_state)
    }

    /// Load state from a connection (used within transactions).
    fn load_state_from_conn(&self, conn: &Connection, task_id: &TaskId) -> AppResult<State> {
        let status_str: String = conn
            .query_row(
                "SELECT internal_status FROM tasks WHERE id = ?1",
                [task_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    AppError::TaskNotFound(task_id.as_str().to_string())
                } else {
                    AppError::Database(e.to_string())
                }
            })?;

        let state: State = status_str
            .parse()
            .map_err(|_| AppError::Database(format!("Invalid status: {}", status_str)))?;

        if state_has_data(&state) {
            if let Some(state_data) = self.load_state_data_inner(conn, task_id)? {
                return Ok(state_data.apply_to_state(state));
            }
        }

        Ok(state)
    }

    /// Persist state within a transaction.
    fn persist_state_in_tx(
        &self,
        conn: &Connection,
        task_id: &TaskId,
        state: &State,
    ) -> AppResult<()> {
        self.persist_state_inner(conn, task_id, state)
    }

    /// Creates a state machine loaded with the current persisted state.
    ///
    /// This is useful when you need to inspect the state machine's state
    /// or manually process events.
    pub fn load_with_state_machine(
        &self,
        task_id: &TaskId,
    ) -> AppResult<(State, TaskStateMachine)> {
        let state = self.load_state(task_id)?;
        let context = TaskContext::new_test(task_id.as_str(), "");
        let machine = TaskStateMachine::new(context);
        Ok((state, machine))
    }

    /// Processes an event and executes a side effect atomically within a transaction.
    ///
    /// This function:
    /// 1. Starts a transaction
    /// 2. Loads the current state
    /// 3. Processes the event through the state machine
    /// 4. Persists the new state
    /// 5. Executes the side effect closure
    /// 6. Commits on success, rolls back on any failure
    ///
    /// If the side effect fails, the state change is rolled back.
    ///
    /// # Arguments
    /// * `task_id` - The task to transition
    /// * `event` - The event to process
    /// * `side_effect` - A closure that receives the old and new states
    ///
    /// # Returns
    /// The new state on success, or an error on failure (with rollback)
    pub fn transition_atomically<F>(
        &self,
        task_id: &TaskId,
        event: &TaskEvent,
        side_effect: F,
    ) -> AppResult<State>
    where
        F: FnOnce(&State, &State) -> AppResult<()>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Start transaction
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Load current state
        let current_state = self.load_state_from_conn(&tx, task_id)?;

        // Create state machine with minimal context for event processing
        let context = TaskContext::new_test(task_id.as_str(), "");
        let mut machine = TaskStateMachine::new(context);

        // Process event
        let response = machine.dispatch(&current_state, event);

        let new_state = match response {
            crate::domain::state_machine::Response::Transition(new_state) => new_state,
            crate::domain::state_machine::Response::NotHandled => {
                return Err(AppError::InvalidTransition {
                    from: current_state.as_str().to_string(),
                    to: format!("event {:?} not handled", event),
                });
            }
            crate::domain::state_machine::Response::Handled => {
                // State didn't change - still run side effect and commit
                side_effect(&current_state, &current_state)?;
                tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
                return Ok(current_state);
            }
        };

        // Persist new state within transaction
        self.persist_state_in_tx(&tx, task_id, &new_state)?;

        // Execute side effect - if this fails, transaction rolls back
        side_effect(&current_state, &new_state)?;

        // Commit transaction
        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;

        Ok(new_state)
    }
}

#[cfg(test)]
#[path = "state_machine_repository_tests.rs"]
mod tests;
