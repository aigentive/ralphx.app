use super::*;

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
    assert!(ReviewPointType::Manual
        .description()
        .contains("user-defined"));
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
    // TaskCategory no longer has a "refactor" variant; complexity is detected
    // via title/description keywords instead. Verify a refactor-like title
    // still triggers complexity detection.
    let task = create_task("Refactor the payment processing module", None);
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
