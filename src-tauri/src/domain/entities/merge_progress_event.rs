// Merge progress event entity - high-level merge/validation progress tracking
// Emitted during task merge operations to provide user-friendly status updates

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level phase of merge/validation process.
///
/// String-based to support dynamic phases derived from project analysis.
/// Structural phases (worktree_setup, programmatic_merge, finalize) are constants;
/// validation phases are derived from the project's validate commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MergePhase(pub String);

impl MergePhase {
    /// Setting up worktree (symlinks, initialization)
    pub const WORKTREE_SETUP: &'static str = "worktree_setup";
    /// Running git merge operation
    pub const PROGRAMMATIC_MERGE: &'static str = "programmatic_merge";
    /// Final merge completion
    pub const FINALIZE: &'static str = "finalize";

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn worktree_setup() -> Self {
        Self(Self::WORKTREE_SETUP.to_string())
    }

    pub fn programmatic_merge() -> Self {
        Self(Self::PROGRAMMATIC_MERGE.to_string())
    }

    pub fn finalize() -> Self {
        Self(Self::FINALIZE.to_string())
    }
}

impl std::fmt::Display for MergePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    /// Current phase of merge/validation (dynamic string ID)
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

/// Phase info for the dynamic phase list sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePhaseInfo {
    /// Unique phase identifier (matches MergePhase string values)
    pub id: String,
    /// Human-readable label for display
    pub label: String,
}

/// Derive a phase ID from a validation command string.
///
/// Generates a stable, URL-safe slug from the command by extracting
/// the meaningful parts (tool + subcommand) and joining with underscores.
/// Examples:
/// - "npm run typecheck" → "npm_run_typecheck"
/// - "cargo clippy --all-targets --all-features -- -D warnings" → "cargo_clippy"
/// - "npx tsc --noEmit" → "npx_tsc"
/// - "cargo test" → "cargo_test"
pub fn derive_phase_id(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return "unknown".to_string();
    }

    // Take the first 1-3 non-flag parts before any `--` separator as the phase ID.
    // `--` marks end of options in most CLI tools (everything after is positional to the tool).
    let mut meaningful: Vec<&str> = Vec::new();
    for part in &parts {
        if *part == "--" {
            break;
        }
        if part.starts_with('-') {
            continue;
        }
        meaningful.push(part);
        if meaningful.len() >= 3 {
            break;
        }
    }

    if meaningful.is_empty() {
        return "unknown".to_string();
    }

    meaningful.join("_").to_lowercase()
}

/// Derive a human-readable label from a validation command string.
///
/// Maps well-known commands to friendly names, falls back to a
/// title-cased version of the phase ID for unknown commands.
pub fn derive_phase_label(command: &str) -> String {
    let cmd_lower = command.to_lowercase();

    // TypeScript type checking
    if cmd_lower.contains("tsc") || (cmd_lower.contains("typecheck") && cmd_lower.contains("npm"))
    {
        return "Type Check".to_string();
    }

    // Linting (generic + well-known linters)
    if cmd_lower.contains("lint")
        || cmd_lower.starts_with("ruff ")
        || cmd_lower.starts_with("eslint ")
        || cmd_lower.starts_with("pylint ")
        || cmd_lower.starts_with("flake8 ")
        || cmd_lower.starts_with("golangci-lint ")
    {
        return "Lint".to_string();
    }

    // Clippy (Rust linting)
    if cmd_lower.contains("cargo") && cmd_lower.contains("clippy") {
        return "Clippy".to_string();
    }

    // Testing
    if cmd_lower.contains("test") || cmd_lower.starts_with("pytest ") || cmd_lower == "pytest" {
        return "Test".to_string();
    }

    // Type checking (Python)
    if cmd_lower.starts_with("mypy ") || cmd_lower == "mypy" || cmd_lower.starts_with("pyright ") {
        return "Type Check".to_string();
    }

    // Format / fmt
    if cmd_lower.contains("fmt") || cmd_lower.contains("format") {
        return "Format".to_string();
    }

    // Build
    if cmd_lower.contains("build") {
        return "Build".to_string();
    }

    // Fallback: title-case the phase ID
    let phase_id = derive_phase_id(command);
    phase_id
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Analysis entry shape used for phase derivation.
/// Only needs `validate` commands — mirrors the subset needed from MergeAnalysisEntry.
#[derive(Debug, Clone, Deserialize)]
pub struct PhaseAnalysisEntry {
    #[serde(default)]
    pub validate: Vec<String>,
}

/// Derive the full ordered phase list from project analysis entries.
///
/// Returns structural phases (worktree_setup, programmatic_merge, finalize)
/// plus one dynamic phase per validate command in the analysis.
pub fn derive_phases_from_analysis(entries: &[PhaseAnalysisEntry]) -> Vec<MergePhaseInfo> {
    let mut phases = vec![
        MergePhaseInfo {
            id: MergePhase::WORKTREE_SETUP.to_string(),
            label: "Worktree Setup".to_string(),
        },
        MergePhaseInfo {
            id: MergePhase::PROGRAMMATIC_MERGE.to_string(),
            label: "Merge".to_string(),
        },
    ];

    for entry in entries {
        for cmd in &entry.validate {
            phases.push(MergePhaseInfo {
                id: derive_phase_id(cmd),
                label: derive_phase_label(cmd),
            });
        }
    }

    phases.push(MergePhaseInfo {
        id: MergePhase::FINALIZE.to_string(),
        label: "Finalize".to_string(),
    });

    phases
}

/// Map a validation command string to a MergePhase.
///
/// Uses `derive_phase_id` to generate the phase identifier dynamically.
/// This replaces the old hardcoded enum-based mapping.
pub fn map_command_to_phase(command: &str) -> MergePhase {
    MergePhase::new(derive_phase_id(command))
}

#[cfg(test)]
#[path = "merge_progress_event_tests.rs"]
mod tests;
