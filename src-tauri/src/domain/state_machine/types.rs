// State machine types - supporting structs for state machine operations
// Includes Blocker tracking and QA failure details

use serde::{Deserialize, Serialize};

/// Represents a task that blocks another task from proceeding.
///
/// Tasks can be blocked by other tasks (dependencies) or by
/// waiting for human input. Once all blockers are resolved,
/// the task can proceed automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    /// The ID of the blocking task or a special identifier for human input
    pub id: String,

    /// Whether this blocker has been resolved
    pub resolved: bool,
}

impl Blocker {
    /// Creates a new unresolved blocker with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resolved: false,
        }
    }

    /// Creates a new blocker for human input requirement
    pub fn human_input(reason: impl Into<String>) -> Self {
        Self {
            id: format!("human:{}", reason.into()),
            resolved: false,
        }
    }

    /// Returns true if this blocker is for human input
    pub fn is_human_input(&self) -> bool {
        self.id.starts_with("human:")
    }

    /// Resolves this blocker
    pub fn resolve(&mut self) {
        self.resolved = true;
    }

    /// Returns a new blocker with resolved = true
    pub fn as_resolved(&self) -> Self {
        Self {
            id: self.id.clone(),
            resolved: true,
        }
    }
}

impl Default for Blocker {
    fn default() -> Self {
        Self {
            id: String::new(),
            resolved: false,
        }
    }
}

/// Represents a single QA test failure with details.
///
/// Used in QaFailedData to track which tests failed and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaFailure {
    /// The name or identifier of the failed test
    pub test_name: String,

    /// The error message or failure reason
    pub error: String,

    /// Optional screenshot path for visual verification failures
    pub screenshot: Option<String>,

    /// Optional expected vs actual values for assertion failures
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl QaFailure {
    /// Creates a new QA failure with just the test name and error
    pub fn new(test_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            error: error.into(),
            screenshot: None,
            expected: None,
            actual: None,
        }
    }

    /// Creates a QA failure with assertion details
    pub fn assertion_failure(
        test_name: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        let expected_str = expected.into();
        let actual_str = actual.into();
        Self {
            test_name: test_name.into(),
            error: format!("Expected '{}' but got '{}'", expected_str, actual_str),
            screenshot: None,
            expected: Some(expected_str),
            actual: Some(actual_str),
        }
    }

    /// Creates a QA failure with a screenshot path
    pub fn visual_failure(
        test_name: impl Into<String>,
        error: impl Into<String>,
        screenshot: impl Into<String>,
    ) -> Self {
        Self {
            test_name: test_name.into(),
            error: error.into(),
            screenshot: Some(screenshot.into()),
            expected: None,
            actual: None,
        }
    }

    /// Adds a screenshot path to this failure
    pub fn with_screenshot(mut self, path: impl Into<String>) -> Self {
        self.screenshot = Some(path.into());
        self
    }
}

impl Default for QaFailure {
    fn default() -> Self {
        Self {
            test_name: String::new(),
            error: String::new(),
            screenshot: None,
            expected: None,
            actual: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // Blocker tests
    // ==================

    #[test]
    fn test_blocker_new_creates_unresolved() {
        let blocker = Blocker::new("task-123");
        assert_eq!(blocker.id, "task-123");
        assert!(!blocker.resolved);
    }

    #[test]
    fn test_blocker_human_input_creates_prefixed_id() {
        let blocker = Blocker::human_input("Need API credentials");
        assert!(blocker.id.starts_with("human:"));
        assert!(blocker.id.contains("Need API credentials"));
        assert!(!blocker.resolved);
    }

    #[test]
    fn test_blocker_is_human_input_true_for_human_blockers() {
        let blocker = Blocker::human_input("Need approval");
        assert!(blocker.is_human_input());
    }

    #[test]
    fn test_blocker_is_human_input_false_for_task_blockers() {
        let blocker = Blocker::new("task-456");
        assert!(!blocker.is_human_input());
    }

    #[test]
    fn test_blocker_resolve_sets_resolved_true() {
        let mut blocker = Blocker::new("task-789");
        assert!(!blocker.resolved);
        blocker.resolve();
        assert!(blocker.resolved);
    }

    #[test]
    fn test_blocker_as_resolved_returns_new_resolved_blocker() {
        let blocker = Blocker::new("task-abc");
        assert!(!blocker.resolved);
        let resolved = blocker.as_resolved();
        assert!(resolved.resolved);
        assert_eq!(resolved.id, blocker.id);
        // Original unchanged
        assert!(!blocker.resolved);
    }

    #[test]
    fn test_blocker_default_creates_empty() {
        let blocker = Blocker::default();
        assert_eq!(blocker.id, "");
        assert!(!blocker.resolved);
    }

    #[test]
    fn test_blocker_clone_works() {
        let blocker = Blocker::new("task-clone");
        let cloned = blocker.clone();
        assert_eq!(blocker, cloned);
    }

    #[test]
    fn test_blocker_equality_works() {
        let b1 = Blocker::new("task-1");
        let b2 = Blocker::new("task-1");
        let b3 = Blocker::new("task-2");
        assert_eq!(b1, b2);
        assert_ne!(b1, b3);
    }

    #[test]
    fn test_blocker_equality_considers_resolved() {
        let b1 = Blocker::new("task-1");
        let b2 = b1.as_resolved();
        assert_ne!(b1, b2);
    }

    #[test]
    fn test_blocker_serializes_to_json() {
        let blocker = Blocker::new("task-json");
        let json = serde_json::to_string(&blocker).unwrap();
        assert!(json.contains("task-json"));
        assert!(json.contains("resolved"));
    }

    #[test]
    fn test_blocker_deserializes_from_json() {
        let json = r#"{"id":"task-parse","resolved":true}"#;
        let blocker: Blocker = serde_json::from_str(json).unwrap();
        assert_eq!(blocker.id, "task-parse");
        assert!(blocker.resolved);
    }

    #[test]
    fn test_blocker_roundtrip_serialization() {
        let blockers = vec![
            Blocker::new("task-1"),
            Blocker::human_input("Need input"),
            Blocker::new("task-2").as_resolved(),
        ];

        for blocker in blockers {
            let json = serde_json::to_string(&blocker).unwrap();
            let restored: Blocker = serde_json::from_str(&json).unwrap();
            assert_eq!(blocker, restored);
        }
    }

    // ==================
    // QaFailure tests
    // ==================

    #[test]
    fn test_qa_failure_new_creates_with_name_and_error() {
        let failure = QaFailure::new("test_login", "Element not found");
        assert_eq!(failure.test_name, "test_login");
        assert_eq!(failure.error, "Element not found");
        assert!(failure.screenshot.is_none());
        assert!(failure.expected.is_none());
        assert!(failure.actual.is_none());
    }

    #[test]
    fn test_qa_failure_assertion_failure_creates_with_expected_actual() {
        let failure = QaFailure::assertion_failure("test_count", "5", "3");
        assert_eq!(failure.test_name, "test_count");
        assert!(failure.error.contains("Expected '5'"));
        assert!(failure.error.contains("got '3'"));
        assert_eq!(failure.expected, Some("5".to_string()));
        assert_eq!(failure.actual, Some("3".to_string()));
    }

    #[test]
    fn test_qa_failure_visual_failure_creates_with_screenshot() {
        let failure = QaFailure::visual_failure(
            "test_button_visible",
            "Button not visible",
            "screenshots/button_test.png",
        );
        assert_eq!(failure.test_name, "test_button_visible");
        assert_eq!(failure.error, "Button not visible");
        assert_eq!(
            failure.screenshot,
            Some("screenshots/button_test.png".to_string())
        );
    }

    #[test]
    fn test_qa_failure_with_screenshot_adds_path() {
        let failure = QaFailure::new("test_render", "Render failed")
            .with_screenshot("screenshots/render.png");
        assert_eq!(
            failure.screenshot,
            Some("screenshots/render.png".to_string())
        );
    }

    #[test]
    fn test_qa_failure_default_creates_empty() {
        let failure = QaFailure::default();
        assert_eq!(failure.test_name, "");
        assert_eq!(failure.error, "");
        assert!(failure.screenshot.is_none());
        assert!(failure.expected.is_none());
        assert!(failure.actual.is_none());
    }

    #[test]
    fn test_qa_failure_clone_works() {
        let failure = QaFailure::new("test_clone", "Clone error");
        let cloned = failure.clone();
        assert_eq!(failure, cloned);
    }

    #[test]
    fn test_qa_failure_equality_works() {
        let f1 = QaFailure::new("test_1", "Error 1");
        let f2 = QaFailure::new("test_1", "Error 1");
        let f3 = QaFailure::new("test_2", "Error 2");
        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_qa_failure_serializes_to_json() {
        let failure = QaFailure::new("test_json", "JSON error")
            .with_screenshot("screen.png");
        let json = serde_json::to_string(&failure).unwrap();
        assert!(json.contains("test_json"));
        assert!(json.contains("JSON error"));
        assert!(json.contains("screen.png"));
    }

    #[test]
    fn test_qa_failure_deserializes_from_json() {
        let json = r#"{
            "test_name": "test_parse",
            "error": "Parse error",
            "screenshot": null,
            "expected": "foo",
            "actual": "bar"
        }"#;
        let failure: QaFailure = serde_json::from_str(json).unwrap();
        assert_eq!(failure.test_name, "test_parse");
        assert_eq!(failure.error, "Parse error");
        assert!(failure.screenshot.is_none());
        assert_eq!(failure.expected, Some("foo".to_string()));
        assert_eq!(failure.actual, Some("bar".to_string()));
    }

    #[test]
    fn test_qa_failure_roundtrip_serialization() {
        let failures = vec![
            QaFailure::new("test_1", "Error 1"),
            QaFailure::assertion_failure("test_2", "a", "b"),
            QaFailure::visual_failure("test_3", "Visual fail", "screen.png"),
            QaFailure::new("test_4", "Error 4").with_screenshot("img.png"),
        ];

        for failure in failures {
            let json = serde_json::to_string(&failure).unwrap();
            let restored: QaFailure = serde_json::from_str(&json).unwrap();
            assert_eq!(failure, restored);
        }
    }

    #[test]
    fn test_qa_failure_debug_format() {
        let failure = QaFailure::new("test_debug", "Debug error");
        let debug_str = format!("{:?}", failure);
        assert!(debug_str.contains("QaFailure"));
        assert!(debug_str.contains("test_debug"));
    }
}
