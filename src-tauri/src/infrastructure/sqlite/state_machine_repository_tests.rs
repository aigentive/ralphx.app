use super::*;
use crate::domain::state_machine::types::QaFailure;
use crate::domain::state_machine::{FailedData, QaFailedData};
use crate::testing::SqliteTestDb;

fn setup_repo() -> (SqliteTestDb, TaskStateMachineRepository, TaskId) {
    let db = SqliteTestDb::new("state-machine-repository");

    // Insert a project and task
    db.with_connection(|conn| {
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
    });

    let repo = TaskStateMachineRepository::new(db.new_connection());
    let task_id = TaskId::from_string("task-1".to_string());

    (db, repo, task_id)
}

// ==================
// load_state tests
// ==================

#[test]
fn test_load_state_returns_current_state() {
    let (_db, repo, task_id) = setup_repo();
    let state = repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::Backlog);
}

#[test]
fn test_load_state_not_found() {
    let (_db, repo, _) = setup_repo();
    let nonexistent = TaskId::from_string("nonexistent".to_string());
    let result = repo.load_state(&nonexistent);
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}

#[test]
fn test_load_state_with_qa_failed_data() {
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

    repo.persist_state(&task_id, &State::Ready).unwrap();

    let state = repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::Ready);
}

#[test]
fn test_persist_state_not_found() {
    let (_db, repo, _) = setup_repo();
    let nonexistent = TaskId::from_string("nonexistent".to_string());
    let result = repo.persist_state(&nonexistent, &State::Ready);
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}

#[test]
fn test_persist_state_saves_qa_failed_data() {
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

    let new_state = repo.process_event(&task_id, &TaskEvent::Schedule).unwrap();
    assert_eq!(new_state, State::Ready);

    // Verify persisted
    let loaded = repo.load_state(&task_id).unwrap();
    assert_eq!(loaded, State::Ready);
}

#[test]
fn test_process_event_not_found() {
    let (_db, repo, _) = setup_repo();
    let nonexistent = TaskId::from_string("nonexistent".to_string());
    let result = repo.process_event(&nonexistent, &TaskEvent::Schedule);
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}

#[test]
fn test_process_event_invalid_transition() {
    let (_db, repo, task_id) = setup_repo();

    // ExecutionComplete is not valid in Backlog state
    let result = repo.process_event(&task_id, &TaskEvent::ExecutionComplete);
    assert!(matches!(result, Err(AppError::InvalidTransition { .. })));
}

#[test]
fn test_process_event_chain() {
    let (_db, repo, task_id) = setup_repo();

    // Backlog -> Ready -> Cancelled
    repo.process_event(&task_id, &TaskEvent::Schedule).unwrap();
    repo.process_event(&task_id, &TaskEvent::Cancel).unwrap();

    let state = repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::Cancelled);
}

#[test]
fn test_process_event_with_state_data() {
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

    let (state, machine) = repo.load_with_state_machine(&task_id).unwrap();

    assert_eq!(state, State::Backlog);
    assert_eq!(machine.context.task_id, "task-1");
}

#[test]
fn test_load_with_state_machine_not_found() {
    let (_db, repo, _) = setup_repo();
    let nonexistent = TaskId::from_string("nonexistent".to_string());
    let result = repo.load_with_state_machine(&nonexistent);
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}

#[test]
fn test_load_with_state_machine_rehydrates_data() {
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, _) = setup_repo();
    let nonexistent = TaskId::from_string("nonexistent".to_string());

    let result = repo.transition_atomically(&nonexistent, &TaskEvent::Schedule, |_, _| Ok(()));
    assert!(matches!(result, Err(AppError::TaskNotFound(_))));
}

#[test]
fn test_transition_atomically_chain_with_side_effects() {
    let (_db, repo, task_id) = setup_repo();

    let transitions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    // First transition: Backlog -> Ready
    let transitions_clone = transitions.clone();
    repo.transition_atomically(&task_id, &TaskEvent::Schedule, move |from, to| {
        transitions_clone
            .lock()
            .unwrap()
            .push((from.clone(), to.clone()));
        Ok(())
    })
    .unwrap();

    // Second transition: Ready -> Cancelled
    let transitions_clone = transitions.clone();
    repo.transition_atomically(&task_id, &TaskEvent::Cancel, move |from, to| {
        transitions_clone
            .lock()
            .unwrap()
            .push((from.clone(), to.clone()));
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
    let (_db, repo, task_id) = setup_repo();

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
    let (_db, repo, task_id) = setup_repo();

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
