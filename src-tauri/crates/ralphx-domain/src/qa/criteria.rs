// Acceptance Criteria and QA Test Step types
// Used for storing and parsing QA artifacts in the database

use serde::{Deserialize, Serialize};

// ============================================================================
// Acceptance Criteria Type Enum
// ============================================================================

/// Type of acceptance criterion for testing categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceCriteriaType {
    /// Visual verification (UI appearance, layout)
    #[default]
    Visual,
    /// Behavioral verification (user interactions, workflows)
    Behavior,
    /// Data verification (API responses, database state)
    Data,
    /// Accessibility verification (a11y compliance)
    Accessibility,
}

impl AcceptanceCriteriaType {
    /// Get all possible values for the enum
    pub fn all() -> &'static [AcceptanceCriteriaType] {
        &[
            AcceptanceCriteriaType::Visual,
            AcceptanceCriteriaType::Behavior,
            AcceptanceCriteriaType::Data,
            AcceptanceCriteriaType::Accessibility,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AcceptanceCriteriaType::Visual => "visual",
            AcceptanceCriteriaType::Behavior => "behavior",
            AcceptanceCriteriaType::Data => "data",
            AcceptanceCriteriaType::Accessibility => "accessibility",
        }
    }
}

impl std::fmt::Display for AcceptanceCriteriaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Acceptance Criterion
// ============================================================================

/// A single acceptance criterion for a task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    /// Unique identifier (e.g., "AC1", "AC2")
    pub id: String,
    /// Description of what needs to be verified
    pub description: String,
    /// Whether this criterion can be tested automatically
    pub testable: bool,
    /// Type of criterion for categorization
    #[serde(rename = "type")]
    pub criteria_type: AcceptanceCriteriaType,
}

impl AcceptanceCriterion {
    /// Create a new acceptance criterion
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        testable: bool,
        criteria_type: AcceptanceCriteriaType,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            testable,
            criteria_type,
        }
    }

    /// Create a new testable visual criterion
    pub fn visual(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(id, description, true, AcceptanceCriteriaType::Visual)
    }

    /// Create a new testable behavior criterion
    pub fn behavior(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(id, description, true, AcceptanceCriteriaType::Behavior)
    }
}

// ============================================================================
// Acceptance Criteria Collection
// ============================================================================

/// Collection of acceptance criteria for a task
/// This is the top-level structure stored as JSON in the database
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AcceptanceCriteria {
    /// List of acceptance criteria
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

impl AcceptanceCriteria {
    /// Create a new empty acceptance criteria collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Create acceptance criteria from a list
    pub fn from_criteria(criteria: Vec<AcceptanceCriterion>) -> Self {
        Self {
            acceptance_criteria: criteria,
        }
    }

    /// Add a criterion to the collection
    pub fn add(&mut self, criterion: AcceptanceCriterion) {
        self.acceptance_criteria.push(criterion);
    }

    /// Get the number of criteria
    pub fn len(&self) -> usize {
        self.acceptance_criteria.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.acceptance_criteria.is_empty()
    }

    /// Get testable criteria only
    pub fn testable(&self) -> impl Iterator<Item = &AcceptanceCriterion> {
        self.acceptance_criteria.iter().filter(|c| c.testable)
    }

    /// Count testable criteria
    pub fn testable_count(&self) -> usize {
        self.acceptance_criteria
            .iter()
            .filter(|c| c.testable)
            .count()
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
// QA Test Step
// ============================================================================

/// A single test step for QA verification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QATestStep {
    /// Unique identifier (e.g., "QA1", "QA2")
    pub id: String,
    /// Reference to the acceptance criterion being tested
    pub criteria_id: String,
    /// Human-readable description of what this step verifies
    pub description: String,
    /// List of agent-browser commands to execute
    pub commands: Vec<String>,
    /// Expected outcome description
    pub expected: String,
}

impl QATestStep {
    /// Create a new QA test step
    pub fn new(
        id: impl Into<String>,
        criteria_id: impl Into<String>,
        description: impl Into<String>,
        commands: Vec<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            criteria_id: criteria_id.into(),
            description: description.into(),
            commands,
            expected: expected.into(),
        }
    }

    /// Check if this step has any commands
    pub fn has_commands(&self) -> bool {
        !self.commands.is_empty()
    }

    /// Get the number of commands
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

// ============================================================================
// QA Test Steps Collection
// ============================================================================

/// Collection of QA test steps for a task
/// This is the top-level structure stored as JSON in the database
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QATestSteps {
    /// List of test steps
    pub qa_steps: Vec<QATestStep>,
}

impl QATestSteps {
    /// Create a new empty test steps collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Create test steps from a list
    pub fn from_steps(steps: Vec<QATestStep>) -> Self {
        Self { qa_steps: steps }
    }

    /// Add a step to the collection
    pub fn add(&mut self, step: QATestStep) {
        self.qa_steps.push(step);
    }

    /// Get the number of steps
    pub fn len(&self) -> usize {
        self.qa_steps.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.qa_steps.is_empty()
    }

    /// Get steps for a specific criterion
    pub fn for_criterion<'a>(
        &'a self,
        criteria_id: &'a str,
    ) -> impl Iterator<Item = &'a QATestStep> {
        self.qa_steps
            .iter()
            .filter(move |s| s.criteria_id == criteria_id)
    }

    /// Get total command count across all steps
    pub fn total_commands(&self) -> usize {
        self.qa_steps.iter().map(|s| s.commands.len()).sum()
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
// Tests
// ============================================================================

#[cfg(test)]
#[path = "criteria_tests.rs"]
mod tests;
