// Merge progress event entity - high-level merge/validation progress tracking
// Emitted during task merge operations to provide user-friendly status updates

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Global in-memory store for merge progress events keyed by task_id.
/// Used to hydrate frontend on mount (events fire before frontend subscribes).
pub static MERGE_PROGRESS_STORE: LazyLock<DashMap<String, Vec<MergeProgressEvent>>> =
    LazyLock::new(DashMap::new);

/// Global in-memory store for merge phase lists keyed by task_id.
/// Used to hydrate frontend on mount (phase list fires before frontend subscribes).
pub static MERGE_PHASE_LIST_STORE: LazyLock<DashMap<String, Vec<MergePhaseInfo>>> =
    LazyLock::new(DashMap::new);

/// Store a merge progress event in the global hydration store.
/// Updates existing phases (started→passed/failed) by matching on phase,
/// or appends new phases — mirrors frontend dedup logic.
pub fn store_merge_progress(event: &MergeProgressEvent) {
    let mut entry = MERGE_PROGRESS_STORE
        .entry(event.task_id.clone())
        .or_default();
    let events = entry.value_mut();
    if let Some(idx) = events.iter().position(|e| e.phase == event.phase) {
        events[idx] = event.clone();
    } else {
        events.push(event.clone());
    }
}

/// Store a merge phase list in the global hydration store.
pub fn store_merge_phase_list(task_id: &str, phases: Vec<MergePhaseInfo>) {
    MERGE_PHASE_LIST_STORE.insert(task_id.to_string(), phases);
}

/// Clear merge progress data for a task (call when merge completes or is abandoned).
pub fn clear_merge_progress(task_id: &str) {
    MERGE_PROGRESS_STORE.remove(task_id);
    MERGE_PHASE_LIST_STORE.remove(task_id);
}

/// High-level phase of merge/validation process.
///
/// String-based to support dynamic phases derived from project analysis.
/// Structural phases (worktree_setup, programmatic_merge, finalize) are constants;
/// validation phases are derived from the project's validate commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MergePhase(pub String);

impl MergePhase {
    // --- Pre-merge pipeline phases ---
    /// Loading task/project and resolving branches
    pub const MERGE_PREPARATION: &'static str = "merge_preparation";
    /// Validating plan merge preconditions
    pub const PRECONDITION_CHECK: &'static str = "precondition_check";
    /// Checking if source/target branches are up-to-date
    pub const BRANCH_FRESHNESS: &'static str = "branch_freshness";
    /// Cleaning up stale worktrees/agents from prior attempts
    pub const MERGE_CLEANUP: &'static str = "merge_cleanup";

    // --- Merge execution phases ---
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
    /// Actual shell command (only set for dynamic/validation phases)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Static description (only set for structural phases)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    if cmd_lower.contains("tsc") || (cmd_lower.contains("typecheck") && cmd_lower.contains("npm")) {
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
            id: MergePhase::MERGE_PREPARATION.to_string(),
            label: "Preparation".to_string(),
            command: None,
            description: Some("Loading task context and resolving branches".to_string()),
        },
        MergePhaseInfo {
            id: MergePhase::PRECONDITION_CHECK.to_string(),
            label: "Preconditions".to_string(),
            command: None,
            description: Some("Validating merge prerequisites and dependencies".to_string()),
        },
        MergePhaseInfo {
            id: MergePhase::BRANCH_FRESHNESS.to_string(),
            label: "Branch Freshness".to_string(),
            command: None,
            description: Some("Checking source branch is up-to-date with target".to_string()),
        },
        MergePhaseInfo {
            id: MergePhase::MERGE_CLEANUP.to_string(),
            label: "Cleanup".to_string(),
            command: None,
            description: Some("Removing stale worktrees and stopping old agents".to_string()),
        },
        MergePhaseInfo {
            id: MergePhase::WORKTREE_SETUP.to_string(),
            label: "Worktree Setup".to_string(),
            command: None,
            description: Some("Creating isolated worktree for validation".to_string()),
        },
        MergePhaseInfo {
            id: MergePhase::PROGRAMMATIC_MERGE.to_string(),
            label: "Merge".to_string(),
            command: None,
            description: Some("Running git merge/rebase operation".to_string()),
        },
    ];

    for entry in entries {
        for cmd in &entry.validate {
            phases.push(MergePhaseInfo {
                id: derive_phase_id(cmd),
                label: derive_phase_label(cmd),
                command: Some(cmd.clone()),
                description: None,
            });
        }
    }

    phases.push(MergePhaseInfo {
        id: MergePhase::FINALIZE.to_string(),
        label: "Finalize".to_string(),
        command: None,
        description: Some("Publishing merge commit and cleaning up".to_string()),
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
