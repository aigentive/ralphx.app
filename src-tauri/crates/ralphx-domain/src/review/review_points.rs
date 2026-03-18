// Review Points Detection
// Logic for detecting when tasks need review points (human-in-the-loop)
//
// Review point types:
// 1. Before Destructive - Auto-inserted before tasks that delete files/configs
// 2. After Complex - Optional, for tasks marked as complex
// 3. Manual - User-defined review points on specific tasks

use serde::{Deserialize, Serialize};

use crate::entities::Task;

/// Configuration for review point behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewPointConfig {
    /// Auto-insert review point before tasks that delete files or modify configs
    /// Default: true
    pub review_before_destructive: bool,

    /// Auto-insert review point after complex tasks
    /// Default: false
    pub review_after_complex: bool,
}

impl Default for ReviewPointConfig {
    fn default() -> Self {
        Self {
            review_before_destructive: true,
            review_after_complex: false,
        }
    }
}

impl ReviewPointConfig {
    /// Create config with destructive review disabled
    pub fn without_destructive_review() -> Self {
        Self {
            review_before_destructive: false,
            ..Default::default()
        }
    }

    /// Create config with complex review enabled
    pub fn with_complex_review() -> Self {
        Self {
            review_after_complex: true,
            ..Default::default()
        }
    }

    /// Create config with all review points enabled
    pub fn all_enabled() -> Self {
        Self {
            review_before_destructive: true,
            review_after_complex: true,
        }
    }
}

/// Type of review point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPointType {
    /// Auto-inserted before destructive tasks
    BeforeDestructive,
    /// Auto-inserted after complex tasks
    AfterComplex,
    /// Manually set by user
    Manual,
}

impl ReviewPointType {
    /// Get the display name for this review point type
    pub fn display_name(&self) -> &'static str {
        match self {
            ReviewPointType::BeforeDestructive => "Before Destructive",
            ReviewPointType::AfterComplex => "After Complex",
            ReviewPointType::Manual => "Manual",
        }
    }

    /// Get a description of this review point type
    pub fn description(&self) -> &'static str {
        match self {
            ReviewPointType::BeforeDestructive => {
                "This task involves destructive operations and requires human approval"
            }
            ReviewPointType::AfterComplex => {
                "This task is complex and requires human verification after completion"
            }
            ReviewPointType::Manual => "This task has a user-defined review point",
        }
    }
}

/// Keywords that indicate destructive file operations
const DESTRUCTIVE_FILE_KEYWORDS: &[&str] = &[
    "delete", "remove", "rm ", "rm -", "unlink", "drop", "truncate", "purge", "wipe", "erase",
    "destroy", "clean up", "cleanup",
];

/// Keywords that indicate config modifications
const CONFIG_KEYWORDS: &[&str] = &[
    "config",
    "configuration",
    "settings",
    "env",
    ".env",
    "environment",
    "credentials",
    "secret",
    "api key",
    "apikey",
    "api_key",
    "token",
    "password",
    "database",
    "connection string",
];

/// Keywords that indicate modification operations on configs
const CONFIG_MODIFICATION_KEYWORDS: &[&str] = &[
    "modify",
    "change",
    "update",
    "alter",
    "edit",
    "reset",
    "overwrite",
    "replace",
    "migrate",
    "rename",
];

/// Keywords that indicate complex tasks
const COMPLEX_KEYWORDS: &[&str] = &[
    "complex",
    "refactor",
    "rewrite",
    "overhaul",
    "restructure",
    "migrate",
    "migration",
    "breaking change",
    "architectural",
    "major",
    "significant",
    "critical",
    "security",
];

/// Check if a task is destructive (deletes files or modifies configs)
///
/// A task is considered destructive if:
/// - Its title or description contains keywords indicating file deletion
/// - Its title or description indicates config/credential modifications
///
/// # Arguments
/// * `task` - The task to check
///
/// # Returns
/// `true` if the task appears to be destructive
pub fn is_destructive_task(task: &Task) -> bool {
    let title_lower = task.title.to_lowercase();
    let desc_lower = task
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();

    let combined = format!("{} {}", title_lower, desc_lower);

    // Check for direct file deletion keywords
    for keyword in DESTRUCTIVE_FILE_KEYWORDS {
        if combined.contains(keyword) {
            return true;
        }
    }

    // Check for config modifications (config keyword + modification keyword)
    let has_config_keyword = CONFIG_KEYWORDS.iter().any(|k| combined.contains(k));
    let has_modification_keyword = CONFIG_MODIFICATION_KEYWORDS
        .iter()
        .any(|k| combined.contains(k));

    if has_config_keyword && has_modification_keyword {
        return true;
    }

    false
}

/// Check if a task is complex
///
/// A task is considered complex if:
/// - Its title or description contains complexity-indicating keywords
/// - Its category is "refactor" or similar
///
/// # Arguments
/// * `task` - The task to check
///
/// # Returns
/// `true` if the task appears to be complex
pub fn is_complex_task(task: &Task) -> bool {
    let title_lower = task.title.to_lowercase();
    let desc_lower = task
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();
    let category_lower = task.category.to_string().to_lowercase();

    let combined = format!("{} {} {}", title_lower, desc_lower, category_lower);

    // Check for complexity keywords
    COMPLEX_KEYWORDS.iter().any(|k| combined.contains(k))
}

/// Determine if a review point should be auto-inserted for a task
///
/// # Arguments
/// * `task` - The task to check
/// * `config` - The review point configuration
///
/// # Returns
/// `Some(ReviewPointType)` if a review point should be inserted, `None` otherwise
pub fn should_auto_insert_review_point(
    task: &Task,
    config: &ReviewPointConfig,
) -> Option<ReviewPointType> {
    // Check for destructive tasks first (higher priority)
    if config.review_before_destructive && is_destructive_task(task) {
        return Some(ReviewPointType::BeforeDestructive);
    }

    // Check for complex tasks
    if config.review_after_complex && is_complex_task(task) {
        return Some(ReviewPointType::AfterComplex);
    }

    None
}

/// Determine the review point type for a task
///
/// This function checks:
/// 1. If the task has a manual review point set
/// 2. If the task is destructive
/// 3. If the task is complex
///
/// # Arguments
/// * `task` - The task to check
/// * `config` - The review point configuration
/// * `has_manual_review_point` - Whether the task has a manual review point
///
/// # Returns
/// `Some(ReviewPointType)` if the task needs a review point, `None` otherwise
pub fn get_review_point_type(
    task: &Task,
    config: &ReviewPointConfig,
    has_manual_review_point: bool,
) -> Option<ReviewPointType> {
    // Manual takes highest priority
    if has_manual_review_point {
        return Some(ReviewPointType::Manual);
    }

    // Then check auto-detected types
    should_auto_insert_review_point(task, config)
}

#[cfg(test)]
#[path = "review_points_tests.rs"]
mod tests;
