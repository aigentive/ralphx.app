// Types for ApplyService

use crate::domain::entities::{InternalStatus, Task, TaskProposalId};

/// Target column for applied tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetColumn {
    /// Draft column - tasks need refinement
    Draft,
    /// Backlog column - confirmed but not scheduled
    Backlog,
    /// Todo/Ready column - ready for execution
    Todo,
}

impl TargetColumn {
    /// Convert to InternalStatus
    pub fn to_status(&self) -> InternalStatus {
        match self {
            TargetColumn::Draft => InternalStatus::Backlog,
            TargetColumn::Backlog => InternalStatus::Backlog,
            TargetColumn::Todo => InternalStatus::Ready,
        }
    }
}

/// Options for applying proposals to the Kanban
#[derive(Debug, Clone)]
pub struct ApplyProposalsOptions {
    /// IDs of proposals to apply
    pub proposal_ids: Vec<TaskProposalId>,
    /// Target column for created tasks
    pub target_column: TargetColumn,
    /// Whether to create task dependencies from proposal dependencies
    pub preserve_dependencies: bool,
}

/// Result of applying proposals
#[derive(Debug, Clone)]
pub struct ApplyProposalsResult {
    /// Tasks that were created
    pub created_tasks: Vec<Task>,
    /// Number of dependencies created
    pub dependencies_created: u32,
    /// Any warnings encountered
    pub warnings: Vec<String>,
    /// Whether the session was marked as converted
    pub session_converted: bool,
}

/// Validation result for selected proposals
#[derive(Debug, Clone)]
pub struct SelectionValidation {
    /// Whether the selection is valid
    pub is_valid: bool,
    /// Circular dependency cycles found (if any)
    pub cycles: Vec<Vec<TaskProposalId>>,
    /// Warning messages
    pub warnings: Vec<String>,
}
