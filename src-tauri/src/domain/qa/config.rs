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
#[path = "config_tests.rs"]
mod tests;
