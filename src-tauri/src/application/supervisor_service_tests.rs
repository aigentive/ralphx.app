use super::*;
use crate::domain::supervisor::ErrorInfo;

fn create_test_service() -> SupervisorService {
    let event_bus = EventBus::new();
    SupervisorService::with_config(
        event_bus,
        SupervisorConfig {
            loop_threshold: 3,
            stuck_threshold: 3,
            ..Default::default()
        },
    )
}

#[tokio::test]
async fn test_supervisor_service_new() {
    let event_bus = EventBus::new();
    let service = SupervisorService::new(event_bus);
    assert_eq!(service.config.token_threshold, 50_000);
}

#[tokio::test]
async fn test_start_monitoring() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    let state = service.get_task_state("task-1").await;
    assert!(state.is_some());
    assert_eq!(state.unwrap().task_id, "task-1");
}

#[tokio::test]
async fn test_stop_monitoring() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;
    service.stop_monitoring("task-1").await;

    let state = service.get_task_state("task-1").await;
    assert!(state.is_none());
}

#[tokio::test]
async fn test_process_task_start_event() {
    let service = create_test_service();
    let event = SupervisorEvent::task_start("task-1", "Test task");

    let action = service.process_event(event).await;
    assert!(action.is_none());

    let state = service.get_task_state("task-1").await;
    assert!(state.is_some());
}

#[tokio::test]
async fn test_process_tool_call_no_loop() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Different tool calls shouldn't trigger loop detection
    let event1 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file1.rs"));
    let event2 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Write", "file2.rs"));
    let event3 = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Edit", "file3.rs"));

    assert!(service.process_event(event1).await.is_none());
    assert!(service.process_event(event2).await.is_none());
    assert!(service.process_event(event3).await.is_none());
}

#[tokio::test]
async fn test_process_tool_call_loop_detected() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Same tool call repeatedly should trigger loop detection
    for i in 0..5 {
        let event = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file.rs"));
        let action = service.process_event(event).await;

        if i >= 3 {
            // After 3+ identical calls, should detect loop
            assert!(action.is_some(), "Expected action after {} calls", i + 1);
        }
    }

    let state = service.get_task_state("task-1").await.unwrap();
    assert!(!state.actions_taken.is_empty());
}

#[tokio::test]
async fn test_process_error_repeating() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    let error_info = ErrorInfo::new("Type error in foo.rs", "compile");

    for i in 0..4 {
        let event = SupervisorEvent::error("task-1", error_info.clone());
        let action = service.process_event(event).await;

        if i >= 2 {
            // After 3+ errors, should take action
            assert!(action.is_some(), "Expected action after {} errors", i + 1);
        }
    }
}

#[tokio::test]
async fn test_process_progress_stuck() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // No progress for multiple checks
    for i in 0..5 {
        let info = ProgressInfo::new(); // has_file_changes and has_new_commits are false
        let event = SupervisorEvent::progress_tick("task-1", info);
        let action = service.process_event(event).await;

        if i >= 4 {
            // After 5+ stuck checks, should take action
            assert!(
                action.is_some(),
                "Expected action after {} stuck checks",
                i + 1
            );
        }
    }
}

#[tokio::test]
async fn test_process_progress_not_stuck() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Progress with file changes
    for _ in 0..5 {
        let mut info = ProgressInfo::new();
        info.has_file_changes = true;
        let event = SupervisorEvent::progress_tick("task-1", info);
        let action = service.process_event(event).await;
        assert!(action.is_none());
    }

    let state = service.get_task_state("task-1").await.unwrap();
    assert_eq!(state.stuck_count, 0);
}

#[tokio::test]
async fn test_process_token_threshold() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    let event = SupervisorEvent::token_threshold("task-1", 60_000, 50_000);
    let action = service.process_event(event).await;

    assert!(action.is_some());
    assert!(matches!(action.unwrap(), SupervisorAction::Pause { .. }));
}

#[tokio::test]
async fn test_process_token_threshold_critical() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
    let action = service.process_event(event).await;

    assert!(action.is_some());
    assert!(matches!(action.unwrap(), SupervisorAction::Kill { .. }));

    assert!(service.is_task_killed("task-1").await);
}

#[tokio::test]
async fn test_process_time_threshold() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // 12 minutes elapsed, 10 minute threshold (just over, should inject guidance)
    let event = SupervisorEvent::time_threshold("task-1", 12, 10);
    let action = service.process_event(event).await;

    assert!(action.is_some());
    assert!(matches!(
        action.unwrap(),
        SupervisorAction::InjectGuidance { .. }
    ));
}

#[tokio::test]
async fn test_process_time_threshold_pause() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // 20 minutes elapsed, 10 minute threshold (2x = pause)
    let event = SupervisorEvent::time_threshold("task-1", 20, 10);
    let action = service.process_event(event).await;

    assert!(action.is_some());
    assert!(matches!(action.unwrap(), SupervisorAction::Pause { .. }));
    assert!(service.is_task_paused("task-1").await);
}

#[tokio::test]
async fn test_resume_paused_task() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Trigger pause (20 min elapsed, 10 min threshold)
    let event = SupervisorEvent::time_threshold("task-1", 20, 10);
    service.process_event(event).await;

    assert!(service.is_task_paused("task-1").await);

    // Resume
    let resumed = service.resume_task("task-1").await;
    assert!(resumed);
    assert!(!service.is_task_paused("task-1").await);
}

#[tokio::test]
async fn test_cannot_resume_killed_task() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Kill the task
    let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
    service.process_event(event).await;

    assert!(service.is_task_killed("task-1").await);

    // Try to resume
    let resumed = service.resume_task("task-1").await;
    assert!(!resumed);
}

#[tokio::test]
async fn test_killed_task_ignores_events() {
    let service = create_test_service();
    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Kill the task
    let event = SupervisorEvent::token_threshold("task-1", 110_000, 50_000);
    service.process_event(event).await;

    // Subsequent events should be ignored
    let event = SupervisorEvent::tool_call("task-1", ToolCallInfo::new("Read", "file.rs"));
    let action = service.process_event(event).await;
    assert!(action.is_none());
}

#[tokio::test]
async fn test_action_handler_called() {
    let event_bus = EventBus::new();
    let mut service = SupervisorService::with_config(
        event_bus,
        SupervisorConfig {
            loop_threshold: 3,
            ..Default::default()
        },
    );

    let action_taken = Arc::new(RwLock::new(false));
    let action_taken_clone = action_taken.clone();

    service.set_action_handler(move |_action, _task_id| {
        let action_taken = action_taken_clone.clone();
        tokio::spawn(async move {
            *action_taken.write().await = true;
        });
    });

    service
        .start_monitoring("task-1".to_string(), "Test task".to_string())
        .await;

    // Trigger an action (20 min elapsed, 10 min threshold)
    let event = SupervisorEvent::time_threshold("task-1", 20, 10);
    service.process_event(event).await;

    // Give the handler time to run
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert!(*action_taken.read().await);
}

#[tokio::test]
async fn test_task_monitor_state() {
    let mut state = TaskMonitorState::new("task-1", "Test task");

    assert_eq!(state.task_id, "task-1");
    assert!(!state.is_paused);
    assert!(!state.is_killed);
    assert_eq!(state.stuck_count, 0);

    state.record_tool_call(ToolCallInfo::new("Read", "file.rs"));
    assert_eq!(state.tool_window.len(), 1);

    state.record_error("Test error");
    assert_eq!(*state.error_counts.get("Test error").unwrap(), 1);

    state.record_progress(ProgressInfo::new(), false);
    assert_eq!(state.stuck_count, 1);

    state.record_progress(ProgressInfo::new(), true);
    assert_eq!(state.stuck_count, 0);
}

#[test]
fn test_supervisor_config_default() {
    let config = SupervisorConfig::default();
    assert_eq!(config.token_threshold, 50_000);
    assert_eq!(config.max_tokens, 100_000);
    assert_eq!(config.time_threshold_seconds, 600);
    assert_eq!(config.progress_interval_seconds, 30);
    assert_eq!(config.loop_threshold, 3);
    assert_eq!(config.stuck_threshold, 5);
}
