// Review Points Detection
// Logic for detecting when tasks need review points (human-in-the-loop)
//
// Review point types:
// 1. Before Destructive - Auto-inserted before tasks that delete files/configs
// 2. After Complex - Optional, for tasks marked as complex
// 3. Manual - User-defined review points on specific tasks

use serde::{Deserialize, Serialize};

use crate::domain::entities::Task;

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
    "delete",
    "remove",
    "rm ",
    "rm -",
    "unlink",
    "drop",
    "truncate",
    "purge",
    "wipe",
    "erase",
    "destroy",
    "clean up",
    "cleanup",
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
    let category_lower = task.category.to_lowercase();

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
mod tests {
    use super::*;
    use crate::domain::entities::{ProjectId, Task};

    // ===== ReviewPointConfig Tests =====

    #[test]
    fn review_point_config_default() {
        let config = ReviewPointConfig::default();
        assert!(config.review_before_destructive);
        assert!(!config.review_after_complex);
    }

    #[test]
    fn review_point_config_without_destructive_review() {
        let config = ReviewPointConfig::without_destructive_review();
        assert!(!config.review_before_destructive);
        assert!(!config.review_after_complex);
    }

    #[test]
    fn review_point_config_with_complex_review() {
        let config = ReviewPointConfig::with_complex_review();
        assert!(config.review_before_destructive);
        assert!(config.review_after_complex);
    }

    #[test]
    fn review_point_config_all_enabled() {
        let config = ReviewPointConfig::all_enabled();
        assert!(config.review_before_destructive);
        assert!(config.review_after_complex);
    }

    #[test]
    fn review_point_config_serialize() {
        let config = ReviewPointConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"review_before_destructive\":true"));
        assert!(json.contains("\"review_after_complex\":false"));
    }

    #[test]
    fn review_point_config_deserialize() {
        let json = r#"{"review_before_destructive":false,"review_after_complex":true}"#;
        let config: ReviewPointConfig = serde_json::from_str(json).unwrap();
        assert!(!config.review_before_destructive);
        assert!(config.review_after_complex);
    }

    // ===== ReviewPointType Tests =====

    #[test]
    fn review_point_type_display_name() {
        assert_eq!(
            ReviewPointType::BeforeDestructive.display_name(),
            "Before Destructive"
        );
        assert_eq!(
            ReviewPointType::AfterComplex.display_name(),
            "After Complex"
        );
        assert_eq!(ReviewPointType::Manual.display_name(), "Manual");
    }

    #[test]
    fn review_point_type_description() {
        assert!(ReviewPointType::BeforeDestructive
            .description()
            .contains("destructive"));
        assert!(ReviewPointType::AfterComplex
            .description()
            .contains("complex"));
        assert!(ReviewPointType::Manual.description().contains("user-defined"));
    }

    #[test]
    fn review_point_type_serialize() {
        let pt = ReviewPointType::BeforeDestructive;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"before_destructive\"");

        let pt = ReviewPointType::AfterComplex;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"after_complex\"");

        let pt = ReviewPointType::Manual;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"manual\"");
    }

    #[test]
    fn review_point_type_deserialize() {
        let pt: ReviewPointType = serde_json::from_str("\"before_destructive\"").unwrap();
        assert_eq!(pt, ReviewPointType::BeforeDestructive);

        let pt: ReviewPointType = serde_json::from_str("\"after_complex\"").unwrap();
        assert_eq!(pt, ReviewPointType::AfterComplex);

        let pt: ReviewPointType = serde_json::from_str("\"manual\"").unwrap();
        assert_eq!(pt, ReviewPointType::Manual);
    }

    // ===== is_destructive_task Tests =====

    fn create_task(title: &str, description: Option<&str>) -> Task {
        let mut task = Task::new(ProjectId::new(), title.to_string());
        task.description = description.map(|s| s.to_string());
        task
    }

    #[test]
    fn is_destructive_task_delete_in_title() {
        let task = create_task("Delete old log files", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_remove_in_title() {
        let task = create_task("Remove deprecated API endpoint", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_rm_command() {
        let task = create_task("Clean up temp files", Some("Run rm -rf /tmp/cache"));
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_drop_keyword() {
        let task = create_task("Drop unused database tables", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_truncate_keyword() {
        let task = create_task("Truncate log table", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_purge_keyword() {
        let task = create_task("Purge old backups", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_wipe_keyword() {
        let task = create_task("Wipe cache directory", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_cleanup_keyword() {
        let task = create_task("Cleanup build artifacts", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_config_modification() {
        let task = create_task("Update database configuration", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_env_modification() {
        let task = create_task("Modify .env variables", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_credentials_change() {
        let task = create_task("Reset API credentials", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_secret_modification() {
        let task = create_task("Change secret key values", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_settings_update() {
        let task = create_task("Update application settings", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_migrate_config() {
        let task = create_task("Migrate environment configuration", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_description_only() {
        let task = create_task(
            "Update system",
            Some("This will delete all existing user sessions"),
        );
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_case_insensitive() {
        let task = create_task("DELETE User Data", None);
        assert!(is_destructive_task(&task));

        let task = create_task("REMOVE old files", None);
        assert!(is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_not_destructive() {
        let task = create_task("Add new feature", None);
        assert!(!is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_not_destructive_create() {
        let task = create_task("Create configuration file", None);
        assert!(!is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_not_destructive_read() {
        let task = create_task("Read settings from database", None);
        assert!(!is_destructive_task(&task));
    }

    #[test]
    fn is_destructive_task_config_read_only() {
        // Reading config without modification keywords is not destructive
        let task = create_task("Read configuration values", None);
        assert!(!is_destructive_task(&task));
    }

    // ===== is_complex_task Tests =====

    #[test]
    fn is_complex_task_refactor_keyword() {
        let task = create_task("Refactor authentication module", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_rewrite_keyword() {
        let task = create_task("Rewrite the entire API layer", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_migration_keyword() {
        let task = create_task("Database migration to new schema", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_breaking_change() {
        let task = create_task("Implement breaking change to user API", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_architectural_keyword() {
        let task = create_task("Architectural changes to state management", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_security_keyword() {
        let task = create_task("Security audit and fixes", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_critical_keyword() {
        let task = create_task("Fix critical performance issue", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_major_keyword() {
        let task = create_task("Major update to payment processing", None);
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_category_refactor() {
        let mut task = create_task("Update code", None);
        task.category = "refactor".to_string();
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_description_complex() {
        let task = create_task(
            "Update module",
            Some("This is a complex change that affects multiple systems"),
        );
        assert!(is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_not_complex() {
        let task = create_task("Add button to UI", None);
        assert!(!is_complex_task(&task));
    }

    #[test]
    fn is_complex_task_simple_feature() {
        let task = create_task("Implement new feature", None);
        assert!(!is_complex_task(&task));
    }

    // ===== should_auto_insert_review_point Tests =====

    #[test]
    fn should_auto_insert_review_point_destructive_enabled() {
        let task = create_task("Delete all temp files", None);
        let config = ReviewPointConfig::default();
        let result = should_auto_insert_review_point(&task, &config);
        assert_eq!(result, Some(ReviewPointType::BeforeDestructive));
    }

    #[test]
    fn should_auto_insert_review_point_destructive_disabled() {
        let task = create_task("Delete all temp files", None);
        let config = ReviewPointConfig::without_destructive_review();
        let result = should_auto_insert_review_point(&task, &config);
        assert_eq!(result, None);
    }

    #[test]
    fn should_auto_insert_review_point_complex_enabled() {
        let task = create_task("Refactor entire codebase", None);
        // Use config without destructive but with complex enabled
        let config = ReviewPointConfig {
            review_before_destructive: false,
            review_after_complex: true,
        };
        let result = should_auto_insert_review_point(&task, &config);
        assert_eq!(result, Some(ReviewPointType::AfterComplex));
    }

    #[test]
    fn should_auto_insert_review_point_complex_disabled() {
        let task = create_task("Refactor entire codebase", None);
        let config = ReviewPointConfig::default(); // complex disabled by default
        let result = should_auto_insert_review_point(&task, &config);
        assert_eq!(result, None);
    }

    #[test]
    fn should_auto_insert_review_point_destructive_over_complex() {
        // Task is both destructive and complex
        let task = create_task(
            "Refactor and delete old modules",
            Some("Major rewrite that removes deprecated code"),
        );
        let config = ReviewPointConfig::all_enabled();
        let result = should_auto_insert_review_point(&task, &config);
        // Destructive takes priority
        assert_eq!(result, Some(ReviewPointType::BeforeDestructive));
    }

    #[test]
    fn should_auto_insert_review_point_none() {
        let task = create_task("Add new button", None);
        let config = ReviewPointConfig::all_enabled();
        let result = should_auto_insert_review_point(&task, &config);
        assert_eq!(result, None);
    }

    // ===== get_review_point_type Tests =====

    #[test]
    fn get_review_point_type_manual_priority() {
        // Manual should take priority over auto-detected
        let task = create_task("Delete all files", None);
        let config = ReviewPointConfig::all_enabled();
        let result = get_review_point_type(&task, &config, true);
        assert_eq!(result, Some(ReviewPointType::Manual));
    }

    #[test]
    fn get_review_point_type_auto_detected() {
        let task = create_task("Delete all files", None);
        let config = ReviewPointConfig::all_enabled();
        let result = get_review_point_type(&task, &config, false);
        assert_eq!(result, Some(ReviewPointType::BeforeDestructive));
    }

    #[test]
    fn get_review_point_type_complex_fallback() {
        let task = create_task("Major refactoring", None);
        let config = ReviewPointConfig::all_enabled();
        let result = get_review_point_type(&task, &config, false);
        assert_eq!(result, Some(ReviewPointType::AfterComplex));
    }

    #[test]
    fn get_review_point_type_none() {
        let task = create_task("Add feature", None);
        let config = ReviewPointConfig::all_enabled();
        let result = get_review_point_type(&task, &config, false);
        assert_eq!(result, None);
    }
}
