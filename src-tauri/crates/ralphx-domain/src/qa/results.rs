// QA Test Results types
// Used for storing and parsing QA test execution results

use serde::{Deserialize, Serialize};

// ============================================================================
// QA Step Status Enum
// ============================================================================

/// Status of a single QA test step execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QAStepStatus {
    /// Step has not started yet
    #[default]
    Pending,
    /// Step is currently executing
    Running,
    /// Step completed successfully
    Passed,
    /// Step failed verification
    Failed,
    /// Step was skipped (dependency failed or manual skip)
    Skipped,
}

impl QAStepStatus {
    /// Get all possible values for the enum
    pub fn all() -> &'static [QAStepStatus] {
        &[
            QAStepStatus::Pending,
            QAStepStatus::Running,
            QAStepStatus::Passed,
            QAStepStatus::Failed,
            QAStepStatus::Skipped,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            QAStepStatus::Pending => "pending",
            QAStepStatus::Running => "running",
            QAStepStatus::Passed => "passed",
            QAStepStatus::Failed => "failed",
            QAStepStatus::Skipped => "skipped",
        }
    }

    /// Check if the step is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            QAStepStatus::Passed | QAStepStatus::Failed | QAStepStatus::Skipped
        )
    }

    /// Check if the step passed
    pub fn is_passed(&self) -> bool {
        matches!(self, QAStepStatus::Passed)
    }

    /// Check if the step failed
    pub fn is_failed(&self) -> bool {
        matches!(self, QAStepStatus::Failed)
    }
}

impl std::fmt::Display for QAStepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// QA Overall Status Enum
// ============================================================================

/// Overall status of QA test execution for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QAOverallStatus {
    /// Tests have not started
    #[default]
    Pending,
    /// Tests are currently running
    Running,
    /// All tests passed
    Passed,
    /// One or more tests failed
    Failed,
}

impl QAOverallStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            QAOverallStatus::Pending => "pending",
            QAOverallStatus::Running => "running",
            QAOverallStatus::Passed => "passed",
            QAOverallStatus::Failed => "failed",
        }
    }

    /// Check if testing is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, QAOverallStatus::Passed | QAOverallStatus::Failed)
    }
}

impl std::fmt::Display for QAOverallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// QA Step Result
// ============================================================================

/// Result of executing a single QA test step
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QAStepResult {
    /// Reference to the QA step ID
    pub step_id: String,
    /// Current status of this step
    pub status: QAStepStatus,
    /// Path to screenshot captured during this step (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// Actual observed value (for comparison failures)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    /// Expected value (for comparison failures)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Error message if step failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl QAStepResult {
    /// Create a new pending step result
    pub fn pending(step_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            status: QAStepStatus::Pending,
            screenshot: None,
            actual: None,
            expected: None,
            error: None,
        }
    }

    /// Create a passed step result
    pub fn passed(step_id: impl Into<String>, screenshot: Option<String>) -> Self {
        Self {
            step_id: step_id.into(),
            status: QAStepStatus::Passed,
            screenshot,
            actual: None,
            expected: None,
            error: None,
        }
    }

    /// Create a failed step result
    pub fn failed(
        step_id: impl Into<String>,
        error: impl Into<String>,
        screenshot: Option<String>,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            status: QAStepStatus::Failed,
            screenshot,
            actual: None,
            expected: None,
            error: Some(error.into()),
        }
    }

    /// Create a failed step result with expected/actual comparison
    pub fn failed_comparison(
        step_id: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
        screenshot: Option<String>,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            status: QAStepStatus::Failed,
            screenshot,
            actual: Some(actual.into()),
            expected: Some(expected.into()),
            error: None,
        }
    }

    /// Create a skipped step result
    pub fn skipped(step_id: impl Into<String>, reason: Option<String>) -> Self {
        Self {
            step_id: step_id.into(),
            status: QAStepStatus::Skipped,
            screenshot: None,
            actual: None,
            expected: None,
            error: reason,
        }
    }

    /// Mark this result as running
    pub fn mark_running(&mut self) {
        self.status = QAStepStatus::Running;
    }

    /// Mark this result as passed
    pub fn mark_passed(&mut self, screenshot: Option<String>) {
        self.status = QAStepStatus::Passed;
        self.screenshot = screenshot;
        self.error = None;
    }

    /// Mark this result as failed
    pub fn mark_failed(&mut self, error: String, screenshot: Option<String>) {
        self.status = QAStepStatus::Failed;
        self.screenshot = screenshot;
        self.error = Some(error);
    }
}

// ============================================================================
// QA Results Totals
// ============================================================================

/// Summary counts for QA test results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct QAResultsTotals {
    /// Total number of test steps
    pub total_steps: usize,
    /// Number of passed steps
    pub passed_steps: usize,
    /// Number of failed steps
    pub failed_steps: usize,
    /// Number of skipped steps
    pub skipped_steps: usize,
}

impl QAResultsTotals {
    /// Create new totals from step results
    pub fn from_results(results: &[QAStepResult]) -> Self {
        let mut totals = Self {
            total_steps: results.len(),
            ..Default::default()
        };

        for result in results {
            match result.status {
                QAStepStatus::Passed => totals.passed_steps += 1,
                QAStepStatus::Failed => totals.failed_steps += 1,
                QAStepStatus::Skipped => totals.skipped_steps += 1,
                _ => {}
            }
        }

        totals
    }

    /// Calculate pass rate as a percentage (0.0 - 100.0)
    pub fn pass_rate(&self) -> f64 {
        if self.total_steps == 0 {
            return 0.0;
        }
        (self.passed_steps as f64 / self.total_steps as f64) * 100.0
    }

    /// Check if all steps passed
    pub fn all_passed(&self) -> bool {
        self.passed_steps == self.total_steps && self.total_steps > 0
    }

    /// Check if any steps failed
    pub fn has_failures(&self) -> bool {
        self.failed_steps > 0
    }
}

// ============================================================================
// QA Results
// ============================================================================

/// Complete QA test results for a task
/// This is the top-level structure stored as JSON in the database
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QAResults {
    /// Task ID these results belong to
    pub task_id: String,
    /// Overall test status
    pub overall_status: QAOverallStatus,
    /// Total number of steps
    pub total_steps: usize,
    /// Number of passed steps
    pub passed_steps: usize,
    /// Number of failed steps
    pub failed_steps: usize,
    /// Individual step results
    pub steps: Vec<QAStepResult>,
}

impl QAResults {
    /// Create new pending QA results for a task
    pub fn new(task_id: impl Into<String>, step_ids: Vec<String>) -> Self {
        let steps: Vec<QAStepResult> = step_ids.into_iter().map(QAStepResult::pending).collect();
        let total_steps = steps.len();

        Self {
            task_id: task_id.into(),
            overall_status: QAOverallStatus::Pending,
            total_steps,
            passed_steps: 0,
            failed_steps: 0,
            steps,
        }
    }

    /// Create QA results from completed step results
    pub fn from_results(task_id: impl Into<String>, steps: Vec<QAStepResult>) -> Self {
        let totals = QAResultsTotals::from_results(&steps);
        let overall_status = if totals.all_passed() {
            QAOverallStatus::Passed
        } else if totals.has_failures() {
            QAOverallStatus::Failed
        } else {
            QAOverallStatus::Pending
        };

        Self {
            task_id: task_id.into(),
            overall_status,
            total_steps: totals.total_steps,
            passed_steps: totals.passed_steps,
            failed_steps: totals.failed_steps,
            steps,
        }
    }

    /// Mark testing as running
    pub fn mark_running(&mut self) {
        self.overall_status = QAOverallStatus::Running;
    }

    /// Update a step result and recalculate totals
    pub fn update_step(
        &mut self,
        step_id: &str,
        status: QAStepStatus,
        error: Option<String>,
        screenshot: Option<String>,
    ) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.step_id == step_id) {
            step.status = status;
            step.error = error;
            step.screenshot = screenshot;
        }
        self.recalculate();
    }

    /// Recalculate totals and overall status from steps
    pub fn recalculate(&mut self) {
        let totals = QAResultsTotals::from_results(&self.steps);
        self.total_steps = totals.total_steps;
        self.passed_steps = totals.passed_steps;
        self.failed_steps = totals.failed_steps;

        // Determine overall status
        if totals.all_passed() {
            self.overall_status = QAOverallStatus::Passed;
        } else if totals.has_failures() {
            self.overall_status = QAOverallStatus::Failed;
        } else if self.steps.iter().any(|s| s.status == QAStepStatus::Running) {
            self.overall_status = QAOverallStatus::Running;
        }
        // Keep current status if still pending/running
    }

    /// Get step result by ID
    pub fn get_step(&self, step_id: &str) -> Option<&QAStepResult> {
        self.steps.iter().find(|s| s.step_id == step_id)
    }

    /// Get mutable step result by ID
    pub fn get_step_mut(&mut self, step_id: &str) -> Option<&mut QAStepResult> {
        self.steps.iter_mut().find(|s| s.step_id == step_id)
    }

    /// Check if testing is complete
    pub fn is_complete(&self) -> bool {
        self.overall_status.is_complete()
    }

    /// Check if all tests passed
    pub fn is_passed(&self) -> bool {
        self.overall_status == QAOverallStatus::Passed
    }

    /// Check if any tests failed
    pub fn is_failed(&self) -> bool {
        self.overall_status == QAOverallStatus::Failed
    }

    /// Get all failed steps
    pub fn failed_steps_iter(&self) -> impl Iterator<Item = &QAStepResult> {
        self.steps.iter().filter(|s| s.status.is_failed())
    }

    /// Get all screenshots from step results
    pub fn screenshots(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter_map(|s| s.screenshot.as_deref())
            .collect()
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON string
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ============================================================================
// Wrapper for PRD format (qa_results object)
// ============================================================================

/// Wrapper for the PRD JSON format that has `qa_results` as the top key
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QAResultsWrapper {
    /// The actual QA results
    pub qa_results: QAResults,
}

impl QAResultsWrapper {
    /// Create a new wrapper
    pub fn new(results: QAResults) -> Self {
        Self {
            qa_results: results,
        }
    }

    /// Parse from JSON string (PRD format)
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string (PRD format)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;
