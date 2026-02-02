use serde::{Deserialize, Serialize};

/// Execution settings for task scheduling and automation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSettings {
    /// Maximum number of concurrent tasks that can execute simultaneously
    pub max_concurrent_tasks: u32,
    /// Whether to auto-commit changes after successful task completion
    pub auto_commit: bool,
    /// Whether to pause execution when a task fails
    pub pause_on_failure: bool,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 2,
            auto_commit: true,
            pause_on_failure: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_settings_default() {
        let settings = ExecutionSettings::default();
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[test]
    fn test_execution_settings_serialization() {
        let settings = ExecutionSettings {
            max_concurrent_tasks: 4,
            auto_commit: false,
            pause_on_failure: false,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: ExecutionSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.max_concurrent_tasks, 4);
        assert!(!deserialized.auto_commit);
        assert!(!deserialized.pause_on_failure);
    }

    #[test]
    fn test_execution_settings_clone() {
        let settings = ExecutionSettings {
            max_concurrent_tasks: 3,
            auto_commit: true,
            pause_on_failure: false,
        };

        let cloned = settings.clone();
        assert_eq!(cloned, settings);
    }
}
