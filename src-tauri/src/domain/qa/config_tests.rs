use super::*;

// ==================== QAPrepStatus tests ====================

#[test]
fn test_qa_prep_status_default_is_pending() {
    assert_eq!(QAPrepStatus::default(), QAPrepStatus::Pending);
}

#[test]
fn test_qa_prep_status_as_str() {
    assert_eq!(QAPrepStatus::Pending.as_str(), "pending");
    assert_eq!(QAPrepStatus::Running.as_str(), "running");
    assert_eq!(QAPrepStatus::Completed.as_str(), "completed");
    assert_eq!(QAPrepStatus::Failed.as_str(), "failed");
}

#[test]
fn test_qa_prep_status_display() {
    assert_eq!(format!("{}", QAPrepStatus::Pending), "pending");
    assert_eq!(format!("{}", QAPrepStatus::Running), "running");
}

#[test]
fn test_qa_prep_status_is_complete() {
    assert!(!QAPrepStatus::Pending.is_complete());
    assert!(!QAPrepStatus::Running.is_complete());
    assert!(QAPrepStatus::Completed.is_complete());
    assert!(!QAPrepStatus::Failed.is_complete());
}

#[test]
fn test_qa_prep_status_is_failed() {
    assert!(!QAPrepStatus::Pending.is_failed());
    assert!(!QAPrepStatus::Running.is_failed());
    assert!(!QAPrepStatus::Completed.is_failed());
    assert!(QAPrepStatus::Failed.is_failed());
}

#[test]
fn test_qa_prep_status_serialize() {
    let status = QAPrepStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"running\"");
}

#[test]
fn test_qa_prep_status_deserialize() {
    let status: QAPrepStatus = serde_json::from_str("\"completed\"").unwrap();
    assert_eq!(status, QAPrepStatus::Completed);
}

// ==================== QATestStatus tests ====================

#[test]
fn test_qa_test_status_default_is_pending() {
    assert_eq!(QATestStatus::default(), QATestStatus::Pending);
}

#[test]
fn test_qa_test_status_as_str() {
    assert_eq!(QATestStatus::Pending.as_str(), "pending");
    assert_eq!(QATestStatus::WaitingForPrep.as_str(), "waiting_for_prep");
    assert_eq!(QATestStatus::Running.as_str(), "running");
    assert_eq!(QATestStatus::Passed.as_str(), "passed");
    assert_eq!(QATestStatus::Failed.as_str(), "failed");
}

#[test]
fn test_qa_test_status_display() {
    assert_eq!(
        format!("{}", QATestStatus::WaitingForPrep),
        "waiting_for_prep"
    );
}

#[test]
fn test_qa_test_status_is_terminal() {
    assert!(!QATestStatus::Pending.is_terminal());
    assert!(!QATestStatus::WaitingForPrep.is_terminal());
    assert!(!QATestStatus::Running.is_terminal());
    assert!(QATestStatus::Passed.is_terminal());
    assert!(QATestStatus::Failed.is_terminal());
}

#[test]
fn test_qa_test_status_is_passed() {
    assert!(!QATestStatus::Pending.is_passed());
    assert!(QATestStatus::Passed.is_passed());
    assert!(!QATestStatus::Failed.is_passed());
}

#[test]
fn test_qa_test_status_serialize() {
    let status = QATestStatus::WaitingForPrep;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"waiting_for_prep\"");
}

#[test]
fn test_qa_test_status_deserialize() {
    let status: QATestStatus = serde_json::from_str("\"passed\"").unwrap();
    assert_eq!(status, QATestStatus::Passed);
}

// ==================== QASettings tests ====================

#[test]
fn test_qa_settings_default() {
    let settings = QASettings::default();
    assert!(settings.qa_enabled);
    assert!(settings.auto_qa_for_ui_tasks);
    assert!(!settings.auto_qa_for_api_tasks);
    assert!(settings.qa_prep_enabled);
    assert!(settings.browser_testing_enabled);
    assert_eq!(settings.browser_testing_url, "http://localhost:1420");
}

#[test]
fn test_qa_settings_with_url() {
    let settings = QASettings::with_url("http://localhost:3000");
    assert!(settings.qa_enabled);
    assert_eq!(settings.browser_testing_url, "http://localhost:3000");
}

#[test]
fn test_qa_settings_disabled() {
    let settings = QASettings::disabled();
    assert!(!settings.qa_enabled);
}

#[test]
fn test_qa_settings_should_run_qa_for_category_ui() {
    let settings = QASettings::default();
    assert!(settings.should_run_qa_for_category("ui"));
    assert!(settings.should_run_qa_for_category("component"));
    assert!(settings.should_run_qa_for_category("feature"));
}

#[test]
fn test_qa_settings_should_run_qa_for_category_api() {
    let settings = QASettings::default();
    // Default has auto_qa_for_api_tasks = false
    assert!(!settings.should_run_qa_for_category("api"));
    assert!(!settings.should_run_qa_for_category("backend"));
    assert!(!settings.should_run_qa_for_category("endpoint"));

    // With API QA enabled
    let settings = QASettings {
        auto_qa_for_api_tasks: true,
        ..Default::default()
    };
    assert!(settings.should_run_qa_for_category("api"));
}

#[test]
fn test_qa_settings_should_run_qa_for_unknown_category() {
    let settings = QASettings::default();
    assert!(!settings.should_run_qa_for_category("unknown"));
    assert!(!settings.should_run_qa_for_category("docs"));
}

#[test]
fn test_qa_settings_disabled_blocks_all() {
    let settings = QASettings::disabled();
    assert!(!settings.should_run_qa_for_category("ui"));
    assert!(!settings.should_run_qa_for_category("api"));
    assert!(!settings.should_run_qa_for_category("feature"));
}

#[test]
fn test_qa_settings_serialize() {
    let settings = QASettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"qa_enabled\":true"));
    assert!(json.contains("\"browser_testing_url\":\"http://localhost:1420\""));
}

#[test]
fn test_qa_settings_deserialize() {
    let json = r#"{
        "qa_enabled": false,
        "auto_qa_for_ui_tasks": true,
        "auto_qa_for_api_tasks": true,
        "qa_prep_enabled": false,
        "browser_testing_enabled": true,
        "browser_testing_url": "http://localhost:5173"
    }"#;
    let settings: QASettings = serde_json::from_str(json).unwrap();
    assert!(!settings.qa_enabled);
    assert!(settings.auto_qa_for_api_tasks);
    assert!(!settings.qa_prep_enabled);
    assert_eq!(settings.browser_testing_url, "http://localhost:5173");

}

#[test]
fn test_qa_settings_roundtrip() {
    let original = QASettings::with_url("http://example.com:8080");
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: QASettings = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// ==================== TaskQAConfig tests ====================

#[test]
fn test_task_qa_config_default() {
    let config = TaskQAConfig::default();
    assert!(config.needs_qa.is_none());
    assert_eq!(config.qa_prep_status, QAPrepStatus::Pending);
    assert_eq!(config.qa_test_status, QATestStatus::Pending);
}

#[test]
fn test_task_qa_config_new() {
    let config = TaskQAConfig::new(true);
    assert_eq!(config.needs_qa, Some(true));

    let config = TaskQAConfig::new(false);
    assert_eq!(config.needs_qa, Some(false));
}

#[test]
fn test_task_qa_config_inherit() {
    let config = TaskQAConfig::inherit();
    assert!(config.needs_qa.is_none());
}

#[test]
fn test_task_qa_config_requires_qa_with_override() {
    let global = QASettings::default();

    let config = TaskQAConfig::new(true);
    assert!(config.requires_qa(&global, "unknown"));

    let config = TaskQAConfig::new(false);
    assert!(!config.requires_qa(&global, "ui"));
}

#[test]
fn test_task_qa_config_requires_qa_inherits() {
    let global = QASettings::default();
    let config = TaskQAConfig::inherit();

    assert!(config.requires_qa(&global, "ui"));
    assert!(!config.requires_qa(&global, "api"));
}

#[test]
fn test_task_qa_config_set_prep_status() {
    let mut config = TaskQAConfig::default();
    config.set_prep_status(QAPrepStatus::Running);
    assert_eq!(config.qa_prep_status, QAPrepStatus::Running);
}

#[test]
fn test_task_qa_config_set_test_status() {
    let mut config = TaskQAConfig::default();
    config.set_test_status(QATestStatus::Passed);
    assert_eq!(config.qa_test_status, QATestStatus::Passed);
}

#[test]
fn test_task_qa_config_is_prep_complete() {
    let mut config = TaskQAConfig::default();
    assert!(!config.is_prep_complete());

    config.set_prep_status(QAPrepStatus::Completed);
    assert!(config.is_prep_complete());
}

#[test]
fn test_task_qa_config_is_testing_passed() {
    let mut config = TaskQAConfig::default();
    assert!(!config.is_testing_passed());

    config.set_test_status(QATestStatus::Passed);
    assert!(config.is_testing_passed());
}

#[test]
fn test_task_qa_config_is_testing_failed() {
    let mut config = TaskQAConfig::default();
    assert!(!config.is_testing_failed());

    config.set_test_status(QATestStatus::Failed);
    assert!(config.is_testing_failed());
}

#[test]
fn test_task_qa_config_serialize() {
    let config = TaskQAConfig::new(true);
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"needs_qa\":true"));
    assert!(json.contains("\"qa_prep_status\":\"pending\""));
}

#[test]
fn test_task_qa_config_deserialize() {
    let json = r#"{
        "needs_qa": null,
        "qa_prep_status": "completed",
        "qa_test_status": "running"
    }"#;
    let config: TaskQAConfig = serde_json::from_str(json).unwrap();
    assert!(config.needs_qa.is_none());
    assert_eq!(config.qa_prep_status, QAPrepStatus::Completed);
    assert_eq!(config.qa_test_status, QATestStatus::Running);
}

#[test]
fn test_task_qa_config_roundtrip() {
    let mut original = TaskQAConfig::new(true);
    original.set_prep_status(QAPrepStatus::Completed);
    original.set_test_status(QATestStatus::Passed);

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: TaskQAConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

