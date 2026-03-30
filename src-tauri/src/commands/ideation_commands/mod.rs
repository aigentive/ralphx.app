// Ideation commands module - aggregates all ideation-related submodules

use std::path::{Path, PathBuf};
use crate::domain::entities::TaskProposal;

/// Returns true if the proposal belongs to the local project (not a foreign cross-project proposal).
/// Uses canonicalized path comparison with fallback to raw PathBuf.
pub(crate) fn is_local_proposal(proposal: &TaskProposal, project_dir: &Path) -> bool {
    match &proposal.target_project {
        None => true,
        Some(tp) => {
            let tp_path = std::fs::canonicalize(tp)
                .unwrap_or_else(|_| PathBuf::from(tp));
            tp_path == project_dir
        }
    }
}

mod ideation_commands_apply;
mod ideation_commands_chat;
mod ideation_commands_cross_project;
mod ideation_commands_dependencies;
pub mod ideation_commands_effort;
pub mod ideation_commands_export;
mod ideation_commands_orchestrator;
mod ideation_commands_proposals;
mod ideation_commands_session;
mod ideation_commands_types;

// Re-export all types
pub use ideation_commands_types::*;

// Re-export all commands
pub use ideation_commands_apply::*;
#[doc(hidden)]
pub use ideation_commands_apply::apply_proposals_core;
pub use ideation_commands_chat::*;
pub use ideation_commands_cross_project::*;
pub use ideation_commands_dependencies::*;
pub use ideation_commands_effort::*;
pub use ideation_commands_export::*;
pub use ideation_commands_orchestrator::*;
pub use ideation_commands_proposals::*;
pub use ideation_commands_session::*;
#[doc(hidden)]
pub use ideation_commands_session::create_ideation_session_impl;

// Re-export helper function for tests
pub use ideation_commands_dependencies::build_dependency_graph;
