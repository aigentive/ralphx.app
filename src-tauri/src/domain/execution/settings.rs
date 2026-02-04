use serde::{Deserialize, Serialize};

/// Execution settings for task scheduling and automation
/// Can be stored per-project (with project_id) or as global defaults (project_id = None)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSettings {
    /// Maximum number of concurrent tasks that can execute simultaneously (per-project)
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

/// Global execution settings that apply across all projects
/// Phase 82: Global concurrency cap to prevent system overload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalExecutionSettings {
    /// Maximum total concurrent tasks across ALL projects (hard cap)
    /// Default: 20, UI max: 50
    pub global_max_concurrent: u32,
}

impl Default for GlobalExecutionSettings {
    fn default() -> Self {
        Self {
            global_max_concurrent: 20,
        }
    }
}

impl GlobalExecutionSettings {
    /// Maximum allowed value for global_max_concurrent (UI enforced)
    pub const MAX_ALLOWED: u32 = 50;

    /// Validate and clamp global_max_concurrent to allowed range
    pub fn validate(&self) -> Self {
        Self {
            global_max_concurrent: self.global_max_concurrent.min(Self::MAX_ALLOWED).max(1),
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

    // Phase 82: GlobalExecutionSettings tests

    #[test]
    fn test_global_execution_settings_default() {
        let settings = GlobalExecutionSettings::default();
        assert_eq!(settings.global_max_concurrent, 20);
    }

    #[test]
    fn test_global_execution_settings_validate_within_range() {
        let settings = GlobalExecutionSettings {
            global_max_concurrent: 30,
        };
        let validated = settings.validate();
        assert_eq!(validated.global_max_concurrent, 30);
    }

    #[test]
    fn test_global_execution_settings_validate_clamped_to_max() {
        let settings = GlobalExecutionSettings {
            global_max_concurrent: 100,
        };
        let validated = settings.validate();
        assert_eq!(validated.global_max_concurrent, GlobalExecutionSettings::MAX_ALLOWED);
    }

    #[test]
    fn test_global_execution_settings_validate_clamped_to_min() {
        let settings = GlobalExecutionSettings {
            global_max_concurrent: 0,
        };
        let validated = settings.validate();
        assert_eq!(validated.global_max_concurrent, 1);
    }
}
