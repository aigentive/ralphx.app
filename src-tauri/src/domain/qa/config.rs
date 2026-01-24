// QA Configuration types
// Global and per-task QA settings

use serde::{Deserialize, Serialize};

/// QA Prep Status - tracks the background QA preparation phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QAPrepStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
}

impl QAPrepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QAPrepStatus::Pending => "pending",
            QAPrepStatus::Running => "running",
            QAPrepStatus::Completed => "completed",
            QAPrepStatus::Failed => "failed",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, QAPrepStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, QAPrepStatus::Failed)
    }
}

impl std::fmt::Display for QAPrepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// QA Test Status - tracks the browser testing phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QATestStatus {
    #[default]
    Pending,
    WaitingForPrep,
    Running,
    Passed,
    Failed,
}

impl QATestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QATestStatus::Pending => "pending",
            QATestStatus::WaitingForPrep => "waiting_for_prep",
            QATestStatus::Running => "running",
            QATestStatus::Passed => "passed",
            QATestStatus::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, QATestStatus::Passed | QATestStatus::Failed)
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, QATestStatus::Passed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, QATestStatus::Failed)
    }
}

impl std::fmt::Display for QATestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Global QA settings stored in project settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QASettings {
    /// Master toggle for QA system
    pub qa_enabled: bool,
    /// Automatically enable QA for UI-related tasks
    pub auto_qa_for_ui_tasks: bool,
    /// Automatically enable QA for API tasks
    pub auto_qa_for_api_tasks: bool,
    /// Enable QA Prep phase (background acceptance criteria generation)
    pub qa_prep_enabled: bool,
    /// Enable browser-based testing
    pub browser_testing_enabled: bool,
    /// URL for browser testing (typically dev server)
    pub browser_testing_url: String,
}

impl Default for QASettings {
    fn default() -> Self {
        Self {
            qa_enabled: true,
            auto_qa_for_ui_tasks: true,
            auto_qa_for_api_tasks: false,
            qa_prep_enabled: true,
            browser_testing_enabled: true,
            browser_testing_url: "http://localhost:1420".to_string(),
        }
    }
}

impl QASettings {
    /// Create new QA settings with custom URL
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            browser_testing_url: url.into(),
            ..Default::default()
        }
    }

    /// Create disabled QA settings
    pub fn disabled() -> Self {
        Self {
            qa_enabled: false,
            ..Default::default()
        }
    }

    /// Check if QA should run for a given task category
    pub fn should_run_qa_for_category(&self, category: &str) -> bool {
        if !self.qa_enabled {
            return false;
        }

        match category {
            "ui" | "component" | "feature" => self.auto_qa_for_ui_tasks,
            "api" | "backend" | "endpoint" => self.auto_qa_for_api_tasks,
            _ => false,
        }
    }
}

/// Per-task QA configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TaskQAConfig {
    /// Override for QA enablement. None means inherit from global settings.
    pub needs_qa: Option<bool>,
    /// Current status of QA preparation phase
    pub qa_prep_status: QAPrepStatus,
    /// Current status of QA testing phase
    pub qa_test_status: QATestStatus,
}

impl TaskQAConfig {
    /// Create a new TaskQAConfig with explicit QA requirement
    pub fn new(needs_qa: bool) -> Self {
        Self {
            needs_qa: Some(needs_qa),
            ..Default::default()
        }
    }

    /// Create a TaskQAConfig that inherits from global settings
    pub fn inherit() -> Self {
        Self::default()
    }

    /// Check if QA is required, given global settings
    pub fn requires_qa(&self, global_settings: &QASettings, task_category: &str) -> bool {
        match self.needs_qa {
            Some(needs) => needs,
            None => global_settings.should_run_qa_for_category(task_category),
        }
    }

    /// Update prep status
    pub fn set_prep_status(&mut self, status: QAPrepStatus) {
        self.qa_prep_status = status;
    }

    /// Update test status
    pub fn set_test_status(&mut self, status: QATestStatus) {
        self.qa_test_status = status;
    }

    /// Check if prep is complete
    pub fn is_prep_complete(&self) -> bool {
        self.qa_prep_status.is_complete()
    }

    /// Check if testing passed
    pub fn is_testing_passed(&self) -> bool {
        self.qa_test_status.is_passed()
    }

    /// Check if testing failed
    pub fn is_testing_failed(&self) -> bool {
        self.qa_test_status.is_failed()
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(format!("{}", QATestStatus::WaitingForPrep), "waiting_for_prep");
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
}
