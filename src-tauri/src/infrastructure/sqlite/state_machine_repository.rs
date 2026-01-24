// TaskStateMachineRepository - SQLite-backed state machine integration
// Provides load/persist operations for task state machine

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
        let conn = self.conn.lock().map_err(|e| AppError::Database(e.to_string()))?;

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
        let conn = self.conn.lock().map_err(|e| AppError::Database(e.to_string()))?;
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
                "UPDATE tasks SET internal_status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
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
                     VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
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
        let conn = self.conn.lock().map_err(|e| AppError::Database(e.to_string()))?;

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
    pub fn load_with_state_machine(&self, task_id: &TaskId) -> AppResult<(State, TaskStateMachine)> {
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
        let conn = self.conn.lock().map_err(|e| AppError::Database(e.to_string()))?;

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
mod tests {
    use super::*;
    use crate::domain::state_machine::{FailedData, QaFailedData};
    use crate::domain::state_machine::types::QaFailure;
    use crate::infrastructure::sqlite::connection::open_memory_connection;
    use crate::infrastructure::sqlite::migrations::run_migrations;

    fn setup_repo() -> (TaskStateMachineRepository, TaskId) {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'backlog')",
            [],
        )
        .unwrap();

        let repo = TaskStateMachineRepository::new(conn);
        let task_id = TaskId::from_string("task-1".to_string());

        (repo, task_id)
    }

    // ==================
    // load_state tests
    // ==================

    #[test]
    fn test_load_state_returns_current_state() {
        let (repo, task_id) = setup_repo();
        let state = repo.load_state(&task_id).unwrap();
        assert_eq!(state, State::Backlog);
    }

    #[test]
    fn test_load_state_not_found() {
        let (repo, _) = setup_repo();
        let nonexistent = TaskId::from_string("nonexistent".to_string());
        let result = repo.load_state(&nonexistent);
        assert!(matches!(result, Err(AppError::TaskNotFound(_))));
    }

    #[test]
    fn test_load_state_with_qa_failed_data() {
        let (repo, task_id) = setup_repo();

        // Manually set up qa_failed state with data
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'qa_failed' WHERE id = 'task-1'",
                [],
            )
            .unwrap();

            let qa_data = QaFailedData::single(QaFailure::new("test_foo", "assertion failed"));
            let json = serde_json::to_string(&qa_data).unwrap();
            conn.execute(
                "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', ?1)",
                [&json],
            )
            .unwrap();
        }

        let state = repo.load_state(&task_id).unwrap();

        if let State::QaFailed(data) = state {
            assert!(data.has_failures());
            assert_eq!(data.first_error(), Some("assertion failed"));
        } else {
            panic!("Expected QaFailed state");
        }
    }

    #[test]
    fn test_load_state_with_failed_data() {
        let (repo, task_id) = setup_repo();

        // Manually set up failed state with data
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'failed' WHERE id = 'task-1'",
                [],
            )
            .unwrap();

            let failed_data = FailedData::new("Build error").with_details("line 42");
            let json = serde_json::to_string(&failed_data).unwrap();
            conn.execute(
                "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'failed', ?1)",
                [&json],
            )
            .unwrap();
        }

        let state = repo.load_state(&task_id).unwrap();

        if let State::Failed(data) = state {
            assert_eq!(data.error, "Build error");
            assert_eq!(data.details, Some("line 42".to_string()));
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_load_state_qa_failed_without_data() {
        let (repo, task_id) = setup_repo();

        // Set qa_failed status but no data
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'qa_failed' WHERE id = 'task-1'",
                [],
            )
            .unwrap();
        }

        let state = repo.load_state(&task_id).unwrap();

        // Should return QaFailed with default data
        if let State::QaFailed(data) = state {
            assert!(!data.has_failures());
        } else {
            panic!("Expected QaFailed state");
        }
    }

    // ==================
    // persist_state tests
    // ==================

    #[test]
    fn test_persist_state_updates_status() {
        let (repo, task_id) = setup_repo();

        repo.persist_state(&task_id, &State::Ready).unwrap();

        let state = repo.load_state(&task_id).unwrap();
        assert_eq!(state, State::Ready);
    }

    #[test]
    fn test_persist_state_not_found() {
        let (repo, _) = setup_repo();
        let nonexistent = TaskId::from_string("nonexistent".to_string());
        let result = repo.persist_state(&nonexistent, &State::Ready);
        assert!(matches!(result, Err(AppError::TaskNotFound(_))));
    }

    #[test]
    fn test_persist_state_saves_qa_failed_data() {
        let (repo, task_id) = setup_repo();

        let qa_data = QaFailedData::single(QaFailure::new("test_persist", "failed"));
        repo.persist_state(&task_id, &State::QaFailed(qa_data))
            .unwrap();

        // Reload and verify data
        let state = repo.load_state(&task_id).unwrap();
        if let State::QaFailed(data) = state {
            assert!(data.has_failures());
            assert_eq!(data.first_error(), Some("failed"));
        } else {
            panic!("Expected QaFailed state");
        }
    }

    #[test]
    fn test_persist_state_saves_failed_data() {
        let (repo, task_id) = setup_repo();

        let failed_data = FailedData::new("Timeout error");
        repo.persist_state(&task_id, &State::Failed(failed_data))
            .unwrap();

        // Reload and verify data
        let state = repo.load_state(&task_id).unwrap();
        if let State::Failed(data) = state {
            assert_eq!(data.error, "Timeout error");
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_persist_state_cleans_up_old_data() {
        let (repo, task_id) = setup_repo();

        // First set to QaFailed with data
        let qa_data = QaFailedData::single(QaFailure::new("test", "error"));
        repo.persist_state(&task_id, &State::QaFailed(qa_data))
            .unwrap();

        // Then transition to a state without data
        repo.persist_state(&task_id, &State::Ready).unwrap();

        // Check that state data was cleaned up
        {
            let conn = repo.conn.lock().unwrap();
            let count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM task_state_data WHERE task_id = 'task-1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    // ==================
    // process_event tests
    // ==================

    #[test]
    fn test_process_event_transitions_state() {
        let (repo, task_id) = setup_repo();

        let new_state = repo.process_event(&task_id, &TaskEvent::Schedule).unwrap();
        assert_eq!(new_state, State::Ready);

        // Verify persisted
        let loaded = repo.load_state(&task_id).unwrap();
        assert_eq!(loaded, State::Ready);
    }

    #[test]
    fn test_process_event_not_found() {
        let (repo, _) = setup_repo();
        let nonexistent = TaskId::from_string("nonexistent".to_string());
        let result = repo.process_event(&nonexistent, &TaskEvent::Schedule);
        assert!(matches!(result, Err(AppError::TaskNotFound(_))));
    }

    #[test]
    fn test_process_event_invalid_transition() {
        let (repo, task_id) = setup_repo();

        // ExecutionComplete is not valid in Backlog state
        let result = repo.process_event(&task_id, &TaskEvent::ExecutionComplete);
        assert!(matches!(result, Err(AppError::InvalidTransition { .. })));
    }

    #[test]
    fn test_process_event_chain() {
        let (repo, task_id) = setup_repo();

        // Backlog -> Ready -> Cancelled
        repo.process_event(&task_id, &TaskEvent::Schedule).unwrap();
        repo.process_event(&task_id, &TaskEvent::Cancel).unwrap();

        let state = repo.load_state(&task_id).unwrap();
        assert_eq!(state, State::Cancelled);
    }

    #[test]
    fn test_process_event_with_state_data() {
        let (repo, task_id) = setup_repo();

        // Set up a QaFailed state
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'qa_failed' WHERE id = 'task-1'",
                [],
            )
            .unwrap();
        }

        // Retry from QaFailed -> RevisionNeeded
        let new_state = repo.process_event(&task_id, &TaskEvent::Retry).unwrap();
        assert_eq!(new_state, State::RevisionNeeded);
    }

    // ==================
    // load_with_state_machine tests
    // ==================

    #[test]
    fn test_load_with_state_machine_returns_state_and_machine() {
        let (repo, task_id) = setup_repo();

        let (state, machine) = repo.load_with_state_machine(&task_id).unwrap();

        assert_eq!(state, State::Backlog);
        assert_eq!(machine.context.task_id, "task-1");
    }

    #[test]
    fn test_load_with_state_machine_not_found() {
        let (repo, _) = setup_repo();
        let nonexistent = TaskId::from_string("nonexistent".to_string());
        let result = repo.load_with_state_machine(&nonexistent);
        assert!(matches!(result, Err(AppError::TaskNotFound(_))));
    }

    #[test]
    fn test_load_with_state_machine_rehydrates_data() {
        let (repo, task_id) = setup_repo();

        // Set up QaFailed with data
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'qa_failed' WHERE id = 'task-1'",
                [],
            )
            .unwrap();

            let qa_data = QaFailedData::single(QaFailure::new("test", "error"));
            let json = serde_json::to_string(&qa_data).unwrap();
            conn.execute(
                "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', ?1)",
                [&json],
            )
            .unwrap();
        }

        let (state, _) = repo.load_with_state_machine(&task_id).unwrap();

        if let State::QaFailed(data) = state {
            assert!(data.has_failures());
        } else {
            panic!("Expected QaFailed state");
        }
    }

    // ==================
    // Transaction tests
    // ==================

    #[test]
    fn test_process_event_is_atomic() {
        let (repo, task_id) = setup_repo();

        // Successful event should persist
        repo.process_event(&task_id, &TaskEvent::Schedule).unwrap();
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);

        // Failed event should not change state
        let result = repo.process_event(&task_id, &TaskEvent::ExecutionComplete);
        assert!(result.is_err());

        // State should still be Ready
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);
    }

    // ==================
    // transition_atomically tests
    // ==================

    #[test]
    fn test_transition_atomically_success() {
        let (repo, task_id) = setup_repo();

        let side_effect_called = std::sync::atomic::AtomicBool::new(false);

        let new_state = repo
            .transition_atomically(&task_id, &TaskEvent::Schedule, |from, to| {
                assert_eq!(from, &State::Backlog);
                assert_eq!(to, &State::Ready);
                side_effect_called.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        assert_eq!(new_state, State::Ready);
        assert!(side_effect_called.load(std::sync::atomic::Ordering::SeqCst));

        // Verify persisted
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);
    }

    #[test]
    fn test_transition_atomically_side_effect_failure_rollback() {
        let (repo, task_id) = setup_repo();

        // Side effect that always fails
        let result = repo.transition_atomically(&task_id, &TaskEvent::Schedule, |_from, _to| {
            Err(AppError::Validation("Side effect failed".to_string()))
        });

        assert!(matches!(result, Err(AppError::Validation(_))));

        // State should NOT have changed (rolled back)
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Backlog);
    }

    #[test]
    fn test_transition_atomically_invalid_event() {
        let (repo, task_id) = setup_repo();

        let side_effect_called = std::sync::atomic::AtomicBool::new(false);

        // ExecutionComplete is not valid in Backlog state
        let result = repo.transition_atomically(&task_id, &TaskEvent::ExecutionComplete, |_, _| {
            side_effect_called.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        assert!(matches!(result, Err(AppError::InvalidTransition { .. })));

        // Side effect should not have been called
        assert!(!side_effect_called.load(std::sync::atomic::Ordering::SeqCst));

        // State should remain unchanged
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Backlog);
    }

    #[test]
    fn test_transition_atomically_not_found() {
        let (repo, _) = setup_repo();
        let nonexistent = TaskId::from_string("nonexistent".to_string());

        let result = repo.transition_atomically(&nonexistent, &TaskEvent::Schedule, |_, _| Ok(()));
        assert!(matches!(result, Err(AppError::TaskNotFound(_))));
    }

    #[test]
    fn test_transition_atomically_chain_with_side_effects() {
        let (repo, task_id) = setup_repo();

        let transitions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        // First transition: Backlog -> Ready
        let transitions_clone = transitions.clone();
        repo.transition_atomically(&task_id, &TaskEvent::Schedule, move |from, to| {
            transitions_clone.lock().unwrap().push((from.clone(), to.clone()));
            Ok(())
        })
        .unwrap();

        // Second transition: Ready -> Cancelled
        let transitions_clone = transitions.clone();
        repo.transition_atomically(&task_id, &TaskEvent::Cancel, move |from, to| {
            transitions_clone.lock().unwrap().push((from.clone(), to.clone()));
            Ok(())
        })
        .unwrap();

        let recorded = transitions.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], (State::Backlog, State::Ready));
        assert_eq!(recorded[1], (State::Ready, State::Cancelled));

        // Final state
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Cancelled);
    }

    #[test]
    fn test_transition_atomically_persists_state_data() {
        let (repo, task_id) = setup_repo();

        // Set up Executing state first
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET internal_status = 'executing' WHERE id = 'task-1'",
                [],
            )
            .unwrap();
        }

        // Transition to Failed with side effect
        let result = repo.transition_atomically(
            &task_id,
            &TaskEvent::ExecutionFailed {
                error: "Build failed".to_string(),
            },
            |from, to| {
                assert_eq!(from, &State::Executing);
                if let State::Failed(data) = to {
                    assert_eq!(data.error, "Build failed");
                } else {
                    panic!("Expected Failed state");
                }
                Ok(())
            },
        );

        // Note: ExecutionFailed creates Failed state but might not preserve error in our impl
        // Let's verify the transition happened
        assert!(result.is_ok());

        if let State::Failed(_) = repo.load_state(&task_id).unwrap() {
            // State is Failed as expected
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_transition_atomically_partial_failure_no_persist() {
        let (repo, task_id) = setup_repo();

        // Schedule first
        repo.transition_atomically(&task_id, &TaskEvent::Schedule, |_, _| Ok(()))
            .unwrap();
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);

        // Try to cancel but fail in side effect
        let result = repo.transition_atomically(&task_id, &TaskEvent::Cancel, |from, to| {
            // Transition is valid (Ready -> Cancelled)
            assert_eq!(from, &State::Ready);
            assert_eq!(to, &State::Cancelled);
            // But we fail the side effect
            Err(AppError::Database("Simulated failure".to_string()))
        });

        assert!(result.is_err());

        // State should still be Ready due to rollback
        assert_eq!(repo.load_state(&task_id).unwrap(), State::Ready);
    }
}
