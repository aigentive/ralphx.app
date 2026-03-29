use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::entities::{ScopeDriftStatus, TaskId};

/// Reviewer classification for detected scope expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeDriftClassification {
    /// Scope expanded into nearby files/surfaces that were reasonably adjacent.
    AdjacentScopeExpansion,
    /// Scope expanded because the original plan needed correction to be valid.
    PlanCorrection,
    /// Scope expanded into unrelated drift that should not be silently approved.
    UnrelatedDrift,
}

impl ScopeDriftClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdjacentScopeExpansion => "adjacent_scope_expansion",
            Self::PlanCorrection => "plan_correction",
            Self::UnrelatedDrift => "unrelated_drift",
        }
    }
}

impl std::fmt::Display for ScopeDriftClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ScopeDriftClassification {
    type Err = ParseScopeDriftClassificationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "adjacent_scope_expansion" => Ok(Self::AdjacentScopeExpansion),
            "plan_correction" => Ok(Self::PlanCorrection),
            "unrelated_drift" => Ok(Self::UnrelatedDrift),
            other => Err(ParseScopeDriftClassificationError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseScopeDriftClassificationError(pub String);

impl std::fmt::Display for ParseScopeDriftClassificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid scope drift classification: '{}', expected 'adjacent_scope_expansion', 'plan_correction', or 'unrelated_drift'",
            self.0
        )
    }
}

impl std::error::Error for ParseScopeDriftClassificationError {}

pub fn compute_scope_drift(
    changed_files: &[String],
    planned_scope: &[String],
) -> (ScopeDriftStatus, Vec<String>) {
    if planned_scope.is_empty() {
        return (ScopeDriftStatus::Unbounded, Vec::new());
    }

    let out_of_scope_files = changed_files
        .iter()
        .filter(|path| !matches_planned_scope(path, planned_scope))
        .cloned()
        .collect::<Vec<_>>();

    let status = if out_of_scope_files.is_empty() {
        ScopeDriftStatus::WithinScope
    } else {
        ScopeDriftStatus::ScopeExpansion
    };

    (status, out_of_scope_files)
}

pub fn matches_planned_scope(path: &str, planned_scope: &[String]) -> bool {
    let normalized_path = normalize_scope_path(path);
    planned_scope.iter().any(|entry| {
        let normalized_entry = normalize_scope_path(entry);
        normalized_path == normalized_entry
            || normalized_path
                .strip_prefix(&normalized_entry)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

pub fn normalize_scope_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

pub fn compute_out_of_scope_blocker_fingerprint(
    task_id: &TaskId,
    out_of_scope_files: &[String],
) -> Option<String> {
    if out_of_scope_files.is_empty() {
        return None;
    }

    let mut normalized_paths: Vec<&str> = out_of_scope_files
        .iter()
        .map(String::as_str)
        .filter(|path| !path.trim().is_empty())
        .collect();
    if normalized_paths.is_empty() {
        return None;
    }

    normalized_paths.sort_unstable();
    normalized_paths.dedup();

    let mut hasher = Sha256::new();
    hasher.update(task_id.as_str().as_bytes());
    hasher.update(b"\n");
    hasher.update(normalized_paths.join("\n").as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    Some(format!("ood:{}:{}", task_id.as_str(), &hash[..12]))
}

#[cfg(test)]
#[path = "scope_drift_tests.rs"]
mod tests;
