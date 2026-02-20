use super::*;

#[test]
fn test_execution_settings_default() {
    let settings = ExecutionSettings::default();
    assert_eq!(settings.max_concurrent_tasks, 10);
    assert!(settings.auto_commit);
    assert!(settings.pause_on_failure);
}

#[test]
fn test_execution_settings_serialization() {
    let settings = ExecutionSettings {
        max_concurrent_tasks: 4,
        auto_commit: false,
        pause_on_failure: false,
    };

    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: ExecutionSettings = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.max_concurrent_tasks, 4);
    assert!(!deserialized.auto_commit);
    assert!(!deserialized.pause_on_failure);
}

#[test]
fn test_execution_settings_clone() {
    let settings = ExecutionSettings {
        max_concurrent_tasks: 3,
        auto_commit: true,
        pause_on_failure: false,
    };

    let cloned = settings.clone();
    assert_eq!(cloned, settings);
}

// Phase 82: GlobalExecutionSettings tests

#[test]
fn test_global_execution_settings_default() {
    let settings = GlobalExecutionSettings::default();
    assert_eq!(settings.global_max_concurrent, 20);
}

#[test]
fn test_global_execution_settings_validate_within_range() {
    let settings = GlobalExecutionSettings {
        global_max_concurrent: 30,
    };
    let validated = settings.validate();
    assert_eq!(validated.global_max_concurrent, 30);
}

#[test]
fn test_global_execution_settings_validate_clamped_to_max() {
    let settings = GlobalExecutionSettings {
        global_max_concurrent: 100,
    };
    let validated = settings.validate();
    assert_eq!(
        validated.global_max_concurrent,
        GlobalExecutionSettings::MAX_ALLOWED
    );
}

#[test]
fn test_global_execution_settings_validate_clamped_to_min() {
    let settings = GlobalExecutionSettings {
        global_max_concurrent: 0,
    };
    let validated = settings.validate();
    assert_eq!(validated.global_max_concurrent, 1);
}
