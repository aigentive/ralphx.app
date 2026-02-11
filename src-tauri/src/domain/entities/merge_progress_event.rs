// Merge progress event entity - high-level merge/validation progress tracking
// Emitted during task merge operations to provide user-friendly status updates

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level phase of merge/validation process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergePhase {
    /// Setting up worktree (symlinks, initialization)
    WorktreeSetup,
    /// Running git merge operation
    ProgrammaticMerge,
    /// TypeScript type checking
    Typecheck,
    /// Linting (npm run lint, cargo clippy)
    Lint,
    /// Clippy specifically (cargo clippy)
    Clippy,
    /// Running tests (cargo test, npm test)
    Test,
    /// Final merge completion
    Finalize,
}

impl std::fmt::Display for MergePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergePhase::WorktreeSetup => write!(f, "worktree_setup"),
            MergePhase::ProgrammaticMerge => write!(f, "programmatic_merge"),
            MergePhase::Typecheck => write!(f, "typecheck"),
            MergePhase::Lint => write!(f, "lint"),
            MergePhase::Clippy => write!(f, "clippy"),
            MergePhase::Test => write!(f, "test"),
            MergePhase::Finalize => write!(f, "finalize"),
        }
    }
}

/// Status of a merge phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergePhaseStatus {
    /// Phase has started
    Started,
    /// Phase completed successfully
    Passed,
    /// Phase failed
    Failed,
}

impl std::fmt::Display for MergePhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergePhaseStatus::Started => write!(f, "started"),
            MergePhaseStatus::Passed => write!(f, "passed"),
            MergePhaseStatus::Failed => write!(f, "failed"),
        }
    }
}

/// High-level merge progress event for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeProgressEvent {
    /// Task being merged
    pub task_id: String,
    /// Current phase of merge/validation
    pub phase: MergePhase,
    /// Status of this phase
    pub status: MergePhaseStatus,
    /// Human-readable message about current state
    pub message: String,
    /// When this event occurred
    pub timestamp: DateTime<Utc>,
}

impl MergeProgressEvent {
    /// Create a new merge progress event
    pub fn new(
        task_id: String,
        phase: MergePhase,
        status: MergePhaseStatus,
        message: String,
    ) -> Self {
        Self {
            task_id,
            phase,
            status,
            message,
            timestamp: Utc::now(),
        }
    }
}

/// Map a validation command string to a canonical merge phase
///
/// This function examines the command and maps it to a user-friendly phase name.
/// Commands that don't match known patterns are mapped to `MergePhase::Finalize`.
pub fn map_command_to_phase(command: &str) -> MergePhase {
    let cmd_lower = command.to_lowercase();

    // TypeScript type checking
    if cmd_lower.contains("tsc") || (cmd_lower.contains("typecheck") && cmd_lower.contains("npm")) {
        return MergePhase::Typecheck;
    }

    // Linting (generic)
    if cmd_lower.contains("npm") && cmd_lower.contains("lint") {
        return MergePhase::Lint;
    }

    // Clippy (Rust linting)
    if cmd_lower.contains("cargo") && cmd_lower.contains("clippy") {
        return MergePhase::Clippy;
    }

    // Testing
    if (cmd_lower.contains("cargo") || cmd_lower.contains("npm")) && cmd_lower.contains("test") {
        return MergePhase::Test;
    }

    // Fallback to finalize for unknown commands
    MergePhase::Finalize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_phase_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&MergePhase::WorktreeSetup).unwrap(),
            "\"worktree_setup\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::ProgrammaticMerge).unwrap(),
            "\"programmatic_merge\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::Typecheck).unwrap(),
            "\"typecheck\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::Lint).unwrap(),
            "\"lint\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::Clippy).unwrap(),
            "\"clippy\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::Test).unwrap(),
            "\"test\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhase::Finalize).unwrap(),
            "\"finalize\""
        );
    }

    #[test]
    fn merge_phase_deserializes_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<MergePhase>("\"worktree_setup\"").unwrap(),
            MergePhase::WorktreeSetup
        );
        assert_eq!(
            serde_json::from_str::<MergePhase>("\"programmatic_merge\"").unwrap(),
            MergePhase::ProgrammaticMerge
        );
        assert_eq!(
            serde_json::from_str::<MergePhase>("\"typecheck\"").unwrap(),
            MergePhase::Typecheck
        );
    }

    #[test]
    fn merge_phase_status_serializes() {
        assert_eq!(
            serde_json::to_string(&MergePhaseStatus::Started).unwrap(),
            "\"started\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhaseStatus::Passed).unwrap(),
            "\"passed\""
        );
        assert_eq!(
            serde_json::to_string(&MergePhaseStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn merge_progress_event_new_sets_fields() {
        let event = MergeProgressEvent::new(
            "task-123".to_string(),
            MergePhase::Typecheck,
            MergePhaseStatus::Started,
            "Running type check".to_string(),
        );

        assert_eq!(event.task_id, "task-123");
        assert_eq!(event.phase, MergePhase::Typecheck);
        assert_eq!(event.status, MergePhaseStatus::Started);
        assert_eq!(event.message, "Running type check");
    }

    #[test]
    fn merge_progress_event_sets_timestamp() {
        let before = Utc::now();
        let event = MergeProgressEvent::new(
            "task-123".to_string(),
            MergePhase::Test,
            MergePhaseStatus::Passed,
            "Tests passed".to_string(),
        );
        let after = Utc::now();

        assert!(event.timestamp >= before);
        assert!(event.timestamp <= after);
    }

    #[test]
    fn merge_progress_event_serializes() {
        let event = MergeProgressEvent::new(
            "task-456".to_string(),
            MergePhase::Clippy,
            MergePhaseStatus::Failed,
            "Clippy errors found".to_string(),
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"task_id\":\"task-456\""));
        assert!(json.contains("\"phase\":\"clippy\""));
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"message\":\"Clippy errors found\""));
    }

    #[test]
    fn merge_phase_display() {
        assert_eq!(format!("{}", MergePhase::WorktreeSetup), "worktree_setup");
        assert_eq!(format!("{}", MergePhase::ProgrammaticMerge), "programmatic_merge");
        assert_eq!(format!("{}", MergePhase::Typecheck), "typecheck");
        assert_eq!(format!("{}", MergePhase::Lint), "lint");
        assert_eq!(format!("{}", MergePhase::Clippy), "clippy");
        assert_eq!(format!("{}", MergePhase::Test), "test");
        assert_eq!(format!("{}", MergePhase::Finalize), "finalize");
    }

    #[test]
    fn merge_phase_status_display() {
        assert_eq!(format!("{}", MergePhaseStatus::Started), "started");
        assert_eq!(format!("{}", MergePhaseStatus::Passed), "passed");
        assert_eq!(format!("{}", MergePhaseStatus::Failed), "failed");
    }

    #[test]
    fn merge_phase_equality() {
        assert_eq!(MergePhase::Typecheck, MergePhase::Typecheck);
        assert_ne!(MergePhase::Typecheck, MergePhase::Lint);
    }

    #[test]
    fn merge_phase_status_equality() {
        assert_eq!(MergePhaseStatus::Started, MergePhaseStatus::Started);
        assert_ne!(MergePhaseStatus::Started, MergePhaseStatus::Passed);
    }

    // Phase mapper tests
    #[test]
    fn map_command_npm_typecheck() {
        assert_eq!(
            map_command_to_phase("npm run typecheck"),
            MergePhase::Typecheck
        );
    }

    #[test]
    fn map_command_npx_tsc() {
        assert_eq!(
            map_command_to_phase("npx tsc --noEmit"),
            MergePhase::Typecheck
        );
    }

    #[test]
    fn map_command_npm_lint() {
        assert_eq!(map_command_to_phase("npm run lint"), MergePhase::Lint);
    }

    #[test]
    fn map_command_cargo_clippy() {
        assert_eq!(
            map_command_to_phase("cargo clippy --all-targets --all-features -- -D warnings"),
            MergePhase::Clippy
        );
    }

    #[test]
    fn map_command_cargo_test() {
        assert_eq!(map_command_to_phase("cargo test"), MergePhase::Test);
    }

    #[test]
    fn map_command_npm_test() {
        assert_eq!(map_command_to_phase("npm run test"), MergePhase::Test);
    }

    #[test]
    fn map_command_npm_test_run() {
        assert_eq!(map_command_to_phase("npm run test:run"), MergePhase::Test);
    }

    #[test]
    fn map_command_unknown() {
        assert_eq!(
            map_command_to_phase("some unknown command"),
            MergePhase::Finalize
        );
    }

    #[test]
    fn map_command_case_insensitive() {
        assert_eq!(
            map_command_to_phase("NPM RUN TYPECHECK"),
            MergePhase::Typecheck
        );
        assert_eq!(
            map_command_to_phase("CARGO CLIPPY"),
            MergePhase::Clippy
        );
    }

    #[test]
    fn map_command_with_extra_flags() {
        assert_eq!(
            map_command_to_phase("npm run typecheck -- --strict"),
            MergePhase::Typecheck
        );
        assert_eq!(
            map_command_to_phase("cargo test --lib --release"),
            MergePhase::Test
        );
    }
}
