use crate::domain::entities::{DependencyGraph, TaskProposalId};

/// Result of dependency validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the selection is valid (no cycles)
    pub is_valid: bool,
    /// Cycles found in the selection (if any)
    pub cycles: Vec<Vec<TaskProposalId>>,
    /// Human-readable warning messages
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            cycles: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with cycles
    pub fn invalid_with_cycles(cycles: Vec<Vec<TaskProposalId>>) -> Self {
        let warnings = cycles
            .iter()
            .enumerate()
            .map(|(i, cycle)| {
                format!(
                    "Cycle {}: {} proposals form a circular dependency",
                    i + 1,
                    cycle.len()
                )
            })
            .collect();

        Self {
            is_valid: false,
            cycles,
            warnings,
        }
    }
}

/// Complete dependency analysis for a session
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// The full dependency graph
    pub graph: DependencyGraph,
    /// Root nodes (no dependencies)
    pub roots: Vec<TaskProposalId>,
    /// Leaf nodes (no dependents)
    pub leaves: Vec<TaskProposalId>,
    /// Blocker nodes (have dependents)
    pub blockers: Vec<TaskProposalId>,
}
