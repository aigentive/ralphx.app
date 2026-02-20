use super::*;
use std::sync::Arc;

// Test that traits are object-safe
#[test]
fn test_agent_spawner_is_object_safe() {
    fn _assert_object_safe(_: &dyn AgentSpawner) {}
}

#[test]
fn test_event_emitter_is_object_safe() {
    fn _assert_object_safe(_: &dyn EventEmitter) {}
}

#[test]
fn test_notifier_is_object_safe() {
    fn _assert_object_safe(_: &dyn Notifier) {}
}

#[test]
fn test_dependency_manager_is_object_safe() {
    fn _assert_object_safe(_: &dyn DependencyManager) {}
}

#[test]
fn test_review_starter_is_object_safe() {
    fn _assert_object_safe(_: &dyn ReviewStarter) {}
}

#[test]
fn test_task_scheduler_is_object_safe() {
    fn _assert_object_safe(_: &dyn TaskScheduler) {}
}

#[test]
fn test_traits_can_be_wrapped_in_arc() {
    // This is important for sharing services across threads
    fn _takes_arc_spawner(_: Arc<dyn AgentSpawner>) {}
    fn _takes_arc_emitter(_: Arc<dyn EventEmitter>) {}
    fn _takes_arc_notifier(_: Arc<dyn Notifier>) {}
    fn _takes_arc_manager(_: Arc<dyn DependencyManager>) {}
    fn _takes_arc_review_starter(_: Arc<dyn ReviewStarter>) {}
    fn _takes_arc_task_scheduler(_: Arc<dyn TaskScheduler>) {}
}

#[test]
fn test_traits_can_be_boxed() {
    fn _takes_box_spawner(_: Box<dyn AgentSpawner>) {}
    fn _takes_box_emitter(_: Box<dyn EventEmitter>) {}
    fn _takes_box_notifier(_: Box<dyn Notifier>) {}
    fn _takes_box_manager(_: Box<dyn DependencyManager>) {}
    fn _takes_box_review_starter(_: Box<dyn ReviewStarter>) {}
    fn _takes_box_task_scheduler(_: Box<dyn TaskScheduler>) {}
}

// ReviewStartResult tests
#[test]
fn test_review_start_result_started() {
    let result = ReviewStartResult::Started {
        review_id: "rev-123".to_string(),
    };
    if let ReviewStartResult::Started { review_id } = result {
        assert_eq!(review_id, "rev-123");
    } else {
        panic!("Expected Started variant");
    }
}

#[test]
fn test_review_start_result_disabled() {
    let result = ReviewStartResult::Disabled;
    assert_eq!(result, ReviewStartResult::Disabled);
}

#[test]
fn test_review_start_result_error() {
    let result = ReviewStartResult::Error("Something failed".to_string());
    if let ReviewStartResult::Error(msg) = result {
        assert_eq!(msg, "Something failed");
    } else {
        panic!("Expected Error variant");
    }
}

#[test]
fn test_review_start_result_clone() {
    let result = ReviewStartResult::Started {
        review_id: "rev-1".to_string(),
    };
    let cloned = result.clone();
    assert_eq!(result, cloned);
}

#[test]
fn test_review_start_result_debug() {
    let result = ReviewStartResult::Started {
        review_id: "rev-1".to_string(),
    };
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("Started"));
    assert!(debug_str.contains("rev-1"));
}
