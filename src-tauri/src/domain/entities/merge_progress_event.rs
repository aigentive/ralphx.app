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
    /// Phase skipped (fail-fast: prior step failed)
    Skipped,
}

impl std::fmt::Display for MergePhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergePhaseStatus::Started => write!(f, "started"),
            MergePhaseStatus::Passed => write!(f, "passed"),
            MergePhaseStatus::Failed => write!(f, "failed"),
            MergePhaseStatus::Skipped => write!(f, "skipped"),
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
#[path = "merge_progress_event_tests.rs"]
mod tests;
