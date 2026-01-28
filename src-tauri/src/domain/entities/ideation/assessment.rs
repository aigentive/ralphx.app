//! Priority assessment system

use serde::{Deserialize, Serialize};

use crate::domain::entities::TaskProposalId;
use super::types::{Priority, Complexity};


// ============================================================================
// PriorityAssessment and detailed factor types
// ============================================================================

/// Factor for dependency analysis - tasks that unblock others get higher priority
/// Max score: 30 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DependencyFactor {
    /// Score from 0-30 based on how many tasks this blocks
    pub score: i32,
    /// Number of tasks that depend on this one (blocked by this task)
    pub blocks_count: i32,
    /// Human-readable explanation (e.g., "Blocks 3 other tasks")
    pub reason: String,
}

impl DependencyFactor {
    /// Create a new dependency factor
    pub fn new(score: i32, blocks_count: i32, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 30),
            blocks_count,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 30;

    /// Calculate score based on blocks count
    /// 0 blocks = 0, 1 = 10, 2 = 18, 3 = 24, 4+ = 30
    pub fn calculate(blocks_count: i32) -> Self {
        let score = match blocks_count {
            0 => 0,
            1 => 10,
            2 => 18,
            3 => 24,
            _ => 30,
        };
        let reason = if blocks_count == 0 {
            "Does not block other tasks".to_string()
        } else if blocks_count == 1 {
            "Blocks 1 other task".to_string()
        } else {
            format!("Blocks {} other tasks", blocks_count)
        };
        Self::new(score, blocks_count, reason)
    }
}

/// Factor for critical path analysis - tasks on the longest path get higher priority
/// Max score: 25 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CriticalPathFactor {
    /// Score from 0-25 based on critical path position
    pub score: i32,
    /// Whether this task is on the critical path
    pub is_on_critical_path: bool,
    /// Length of the critical path this task is on
    pub path_length: i32,
    /// Human-readable explanation
    pub reason: String,
}

impl CriticalPathFactor {
    /// Create a new critical path factor
    pub fn new(
        score: i32,
        is_on_critical_path: bool,
        path_length: i32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            score: score.clamp(0, 25),
            is_on_critical_path,
            path_length,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 25;

    /// Calculate score based on critical path analysis
    pub fn calculate(is_on_critical_path: bool, path_length: i32) -> Self {
        if !is_on_critical_path {
            return Self::new(0, false, 0, "Not on critical path");
        }
        // Score based on path length: longer paths = higher priority
        let score = match path_length {
            1 => 10,
            2 => 15,
            3 => 20,
            _ => 25,
        };
        let reason = format!("On critical path of length {}", path_length);
        Self::new(score, true, path_length, reason)
    }
}

/// Factor for business value analysis - keyword-based importance detection
/// Max score: 20 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BusinessValueFactor {
    /// Score from 0-20 based on detected keywords
    pub score: i32,
    /// Keywords detected that indicate importance (e.g., ["MVP", "core", "essential"])
    pub keywords: Vec<String>,
    /// Human-readable explanation
    pub reason: String,
}

impl BusinessValueFactor {
    /// Create a new business value factor
    pub fn new(score: i32, keywords: Vec<String>, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 20),
            keywords,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 20;

    /// Keywords that indicate critical business value
    pub const CRITICAL_KEYWORDS: &'static [&'static str] = &[
        "critical",
        "blocker",
        "blocking",
        "urgent",
        "asap",
        "emergency",
        "must have",
        "must-have",
    ];

    /// Keywords that indicate high business value
    pub const HIGH_KEYWORDS: &'static [&'static str] = &[
        "important",
        "priority",
        "essential",
        "core",
        "mvp",
        "key",
        "crucial",
    ];

    /// Keywords that indicate low business value
    pub const LOW_KEYWORDS: &'static [&'static str] = &[
        "nice to have",
        "nice-to-have",
        "optional",
        "future",
        "later",
        "eventually",
        "if time",
    ];

    /// Calculate score based on keywords found in text
    pub fn calculate(text: &str) -> Self {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();

        // Check for critical keywords (high score)
        for &kw in Self::CRITICAL_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                20,
                detected,
                "Contains critical business value keywords".to_string(),
            );
        }

        // Check for high keywords (medium-high score)
        for &kw in Self::HIGH_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                15,
                detected,
                "Contains high business value keywords".to_string(),
            );
        }

        // Check for low keywords (low score)
        for &kw in Self::LOW_KEYWORDS {
            if text_lower.contains(kw) {
                detected.push(kw.to_string());
            }
        }
        if !detected.is_empty() {
            return Self::new(
                5,
                detected,
                "Contains low priority keywords".to_string(),
            );
        }

        Self::new(10, vec![], "No business value keywords detected".to_string())
    }
}

/// Factor for complexity analysis - simpler tasks score higher (quick wins first)
/// Max score: 15 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ComplexityFactor {
    /// Score from 0-15 (simpler = higher score)
    pub score: i32,
    /// The complexity level
    pub complexity: Complexity,
    /// Human-readable explanation
    pub reason: String,
}

impl ComplexityFactor {
    /// Create a new complexity factor
    pub fn new(score: i32, complexity: Complexity, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 15),
            complexity,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 15;

    /// Calculate score based on complexity (inverse - simpler = higher)
    pub fn calculate(complexity: Complexity) -> Self {
        let (score, reason) = match complexity {
            Complexity::Trivial => (15, "Quick win - trivial task"),
            Complexity::Simple => (12, "Low effort - simple task"),
            Complexity::Moderate => (9, "Moderate complexity"),
            Complexity::Complex => (5, "Complex task - higher effort"),
            Complexity::VeryComplex => (2, "Very complex - significant effort required"),
        };
        Self::new(score, complexity, reason)
    }
}

/// Factor for user hint analysis - explicit urgency signals from user
/// Max score: 10 points
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UserHintFactor {
    /// Score from 0-10 based on detected hints
    pub score: i32,
    /// Hints detected from user input (e.g., ["urgent", "blocker", "ASAP"])
    pub hints: Vec<String>,
    /// Human-readable explanation
    pub reason: String,
}

impl UserHintFactor {
    /// Create a new user hint factor
    pub fn new(score: i32, hints: Vec<String>, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0, 10),
            hints,
            reason: reason.into(),
        }
    }

    /// Maximum possible score for this factor
    pub const MAX_SCORE: i32 = 10;

    /// Urgency hint keywords
    pub const URGENCY_HINTS: &'static [&'static str] = &[
        "urgent",
        "asap",
        "immediately",
        "now",
        "today",
        "deadline",
        "blocker",
        "blocking",
        "priority",
        "first",
    ];

    /// Calculate score based on hints found in user input
    pub fn calculate(text: &str) -> Self {
        let text_lower = text.to_lowercase();
        let mut detected = Vec::new();

        for &hint in Self::URGENCY_HINTS {
            if text_lower.contains(hint) {
                detected.push(hint.to_string());
            }
        }

        if detected.is_empty() {
            return Self::new(0, vec![], "No urgency hints from user".to_string());
        }

        let score = (detected.len() as i32 * 3).min(10);
        let reason = format!("User indicated urgency: {}", detected.join(", "));
        Self::new(score, detected, reason)
    }
}

/// Container for all priority factors used in priority assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PriorityAssessmentFactors {
    /// Dependency factor (0-30 points)
    pub dependency_factor: DependencyFactor,
    /// Critical path factor (0-25 points)
    pub critical_path_factor: CriticalPathFactor,
    /// Business value factor (0-20 points)
    pub business_value_factor: BusinessValueFactor,
    /// Complexity factor (0-15 points)
    pub complexity_factor: ComplexityFactor,
    /// User hint factor (0-10 points)
    pub user_hint_factor: UserHintFactor,
}

impl PriorityAssessmentFactors {
    /// Maximum possible total score (30 + 25 + 20 + 15 + 10 = 100)
    pub const MAX_TOTAL_SCORE: i32 = 100;

    /// Calculate total score from all factors
    pub fn total_score(&self) -> i32 {
        self.dependency_factor.score
            + self.critical_path_factor.score
            + self.business_value_factor.score
            + self.complexity_factor.score
            + self.user_hint_factor.score
    }
}

/// Complete priority assessment result for a task proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityAssessment {
    /// ID of the proposal this assessment is for
    pub proposal_id: TaskProposalId,
    /// Final suggested priority level
    pub suggested_priority: Priority,
    /// Numeric priority score (0-100)
    pub priority_score: i32,
    /// Human-readable explanation of the priority
    pub priority_reason: String,
    /// Detailed breakdown of all factors
    pub factors: PriorityAssessmentFactors,
}

impl PriorityAssessment {
    /// Create a new priority assessment
    pub fn new(
        proposal_id: TaskProposalId,
        factors: PriorityAssessmentFactors,
    ) -> Self {
        let priority_score = factors.total_score();
        let suggested_priority = Self::score_to_priority(priority_score);
        let priority_reason = Self::generate_reason(&factors, priority_score);

        Self {
            proposal_id,
            suggested_priority,
            priority_score,
            priority_reason,
            factors,
        }
    }

    /// Convert a numeric score (0-100) to a Priority level
    /// 80-100: Critical
    /// 60-79: High
    /// 40-59: Medium
    /// 0-39: Low
    pub fn score_to_priority(score: i32) -> Priority {
        match score {
            80..=100 => Priority::Critical,
            60..=79 => Priority::High,
            40..=59 => Priority::Medium,
            _ => Priority::Low,
        }
    }

    /// Generate a human-readable reason from factors
    fn generate_reason(factors: &PriorityAssessmentFactors, score: i32) -> String {
        let mut reasons = Vec::new();

        if factors.dependency_factor.score > 0 {
            reasons.push(factors.dependency_factor.reason.clone());
        }
        if factors.critical_path_factor.score > 10 {
            reasons.push(factors.critical_path_factor.reason.clone());
        }
        if factors.business_value_factor.score >= 15 {
            reasons.push(factors.business_value_factor.reason.clone());
        }
        if factors.complexity_factor.score >= 12 {
            reasons.push(factors.complexity_factor.reason.clone());
        }
        if factors.user_hint_factor.score > 0 {
            reasons.push(factors.user_hint_factor.reason.clone());
        }

        if reasons.is_empty() {
            format!("Standard priority (score: {})", score)
        } else {
            reasons.join("; ")
        }
    }

    /// Create a default/neutral assessment for a proposal
    pub fn neutral(proposal_id: TaskProposalId) -> Self {
        Self::new(proposal_id, PriorityAssessmentFactors::default())
    }
}

