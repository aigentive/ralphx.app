// Review Configuration types
// Global settings for AI and human code review

use serde::{Deserialize, Serialize};

/// Global review settings stored in project settings
///
/// Controls how the review system behaves including:
/// - Whether AI review is enabled
/// - Whether to auto-create fix tasks
/// - Human review requirements
/// - Max fix attempts before giving up
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewSettings {
    /// Master toggle for AI review system
    /// Default: true
    pub ai_review_enabled: bool,

    /// Automatically create fix tasks when AI review fails
    /// If false, failed reviews go to backlog instead
    /// Default: true
    pub ai_review_auto_fix: bool,

    /// Require human approval before executing AI-proposed fix tasks
    /// Default: false
    pub require_fix_approval: bool,

    /// Require human review even after AI approval
    /// If true, AI-approved tasks still need human sign-off
    /// Default: false
    pub require_human_review: bool,

    /// Maximum fix attempts before giving up and moving to backlog
    /// Default: 3
    pub max_fix_attempts: u32,

    /// Maximum revision cycles (review → changes requested → re-execution) before failing
    /// Default: 5
    pub max_revision_cycles: u32,
}

impl Default for ReviewSettings {
    fn default() -> Self {
        Self {
            ai_review_enabled: true,
            ai_review_auto_fix: true,
            require_fix_approval: false,
            require_human_review: false,
            max_fix_attempts: 3,
            max_revision_cycles: 5,
        }
    }
}

impl ReviewSettings {
    /// Create review settings with AI review disabled
    pub fn ai_disabled() -> Self {
        Self {
            ai_review_enabled: false,
            ..Default::default()
        }
    }

    /// Create review settings that require human review
    pub fn with_human_review() -> Self {
        Self {
            require_human_review: true,
            ..Default::default()
        }
    }

    /// Create review settings with fix approval required
    pub fn with_fix_approval() -> Self {
        Self {
            require_fix_approval: true,
            ..Default::default()
        }
    }

    /// Create review settings with custom max fix attempts
    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            max_fix_attempts: max_attempts,
            ..Default::default()
        }
    }

    /// Check if AI review should run
    pub fn should_run_ai_review(&self) -> bool {
        self.ai_review_enabled
    }

    /// Check if fix tasks should be auto-created on review failure
    pub fn should_auto_create_fix(&self) -> bool {
        self.ai_review_auto_fix
    }

    /// Check if human review is required after AI approval
    pub fn needs_human_review(&self) -> bool {
        self.require_human_review
    }

    /// Check if fix tasks need human approval before execution
    pub fn needs_fix_approval(&self) -> bool {
        self.require_fix_approval
    }

    /// Check if we've exceeded the max fix attempts
    pub fn exceeded_max_attempts(&self, attempts: u32) -> bool {
        attempts >= self.max_fix_attempts
    }

    /// Check if we've exceeded the max revision cycles
    pub fn exceeded_max_revisions(&self, revision_count: u32) -> bool {
        revision_count >= self.max_revision_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_settings_default() {
        let settings = ReviewSettings::default();
        assert!(settings.ai_review_enabled);
        assert!(settings.ai_review_auto_fix);
        assert!(!settings.require_fix_approval);
        assert!(!settings.require_human_review);
        assert_eq!(settings.max_fix_attempts, 3);
        assert_eq!(settings.max_revision_cycles, 5);
    }

    #[test]
    fn test_review_settings_ai_disabled() {
        let settings = ReviewSettings::ai_disabled();
        assert!(!settings.ai_review_enabled);
        // Other defaults preserved
        assert!(settings.ai_review_auto_fix);
        assert_eq!(settings.max_fix_attempts, 3);
    }

    #[test]
    fn test_review_settings_with_human_review() {
        let settings = ReviewSettings::with_human_review();
        assert!(settings.require_human_review);
        // Other defaults preserved
        assert!(settings.ai_review_enabled);
    }

    #[test]
    fn test_review_settings_with_fix_approval() {
        let settings = ReviewSettings::with_fix_approval();
        assert!(settings.require_fix_approval);
    }

    #[test]
    fn test_review_settings_with_max_attempts() {
        let settings = ReviewSettings::with_max_attempts(5);
        assert_eq!(settings.max_fix_attempts, 5);
    }

    #[test]
    fn test_should_run_ai_review() {
        let settings = ReviewSettings::default();
        assert!(settings.should_run_ai_review());

        let settings = ReviewSettings::ai_disabled();
        assert!(!settings.should_run_ai_review());
    }

    #[test]
    fn test_should_auto_create_fix() {
        let settings = ReviewSettings::default();
        assert!(settings.should_auto_create_fix());

        let settings = ReviewSettings {
            ai_review_auto_fix: false,
            ..Default::default()
        };
        assert!(!settings.should_auto_create_fix());
    }

    #[test]
    fn test_needs_human_review() {
        let settings = ReviewSettings::default();
        assert!(!settings.needs_human_review());

        let settings = ReviewSettings::with_human_review();
        assert!(settings.needs_human_review());
    }

    #[test]
    fn test_needs_fix_approval() {
        let settings = ReviewSettings::default();
        assert!(!settings.needs_fix_approval());

        let settings = ReviewSettings::with_fix_approval();
        assert!(settings.needs_fix_approval());
    }

    #[test]
    fn test_exceeded_max_attempts() {
        let settings = ReviewSettings::default();
        assert!(!settings.exceeded_max_attempts(0));
        assert!(!settings.exceeded_max_attempts(1));
        assert!(!settings.exceeded_max_attempts(2));
        assert!(settings.exceeded_max_attempts(3));
        assert!(settings.exceeded_max_attempts(5));

        let settings = ReviewSettings::with_max_attempts(1);
        assert!(!settings.exceeded_max_attempts(0));
        assert!(settings.exceeded_max_attempts(1));
    }

    #[test]
    fn test_review_settings_serialize() {
        let settings = ReviewSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"ai_review_enabled\":true"));
        assert!(json.contains("\"ai_review_auto_fix\":true"));
        assert!(json.contains("\"require_fix_approval\":false"));
        assert!(json.contains("\"require_human_review\":false"));
        assert!(json.contains("\"max_fix_attempts\":3"));
    }

    #[test]
    fn test_review_settings_deserialize() {
        let json = r#"{
            "ai_review_enabled": false,
            "ai_review_auto_fix": false,
            "require_fix_approval": true,
            "require_human_review": true,
            "max_fix_attempts": 5,
            "max_revision_cycles": 8
        }"#;
        let settings: ReviewSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.ai_review_enabled);
        assert!(!settings.ai_review_auto_fix);
        assert!(settings.require_fix_approval);
        assert!(settings.require_human_review);
        assert_eq!(settings.max_fix_attempts, 5);
        assert_eq!(settings.max_revision_cycles, 8);
    }

    #[test]
    fn test_review_settings_roundtrip() {
        let original = ReviewSettings {
            ai_review_enabled: true,
            ai_review_auto_fix: false,
            require_fix_approval: true,
            require_human_review: false,
            max_fix_attempts: 7,
            max_revision_cycles: 8,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ReviewSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_review_settings_partial_json_with_defaults() {
        // Test that serde can handle partial JSON with defaults
        let json = r#"{
            "ai_review_enabled": true,
            "ai_review_auto_fix": true,
            "require_fix_approval": false,
            "require_human_review": false,
            "max_fix_attempts": 3,
            "max_revision_cycles": 5
        }"#;
        let settings: ReviewSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings, ReviewSettings::default());
    }

    #[test]
    fn test_exceeded_max_revisions() {
        let settings = ReviewSettings::default();
        assert!(!settings.exceeded_max_revisions(0));
        assert!(!settings.exceeded_max_revisions(1));
        assert!(!settings.exceeded_max_revisions(4));
        assert!(settings.exceeded_max_revisions(5));
        assert!(settings.exceeded_max_revisions(10));

        let settings = ReviewSettings {
            max_revision_cycles: 2,
            ..Default::default()
        };
        assert!(!settings.exceeded_max_revisions(0));
        assert!(!settings.exceeded_max_revisions(1));
        assert!(settings.exceeded_max_revisions(2));
    }
}
