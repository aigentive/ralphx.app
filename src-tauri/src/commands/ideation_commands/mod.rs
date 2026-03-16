// Ideation commands module - aggregates all ideation-related submodules

mod ideation_commands_apply;
mod ideation_commands_chat;
mod ideation_commands_cross_project;
mod ideation_commands_dependencies;
pub mod ideation_commands_export;
mod ideation_commands_orchestrator;
mod ideation_commands_proposals;
mod ideation_commands_session;
mod ideation_commands_types;

// Re-export all types
pub use ideation_commands_types::*;

// Re-export all commands
pub use ideation_commands_apply::*;
pub use ideation_commands_chat::*;
pub use ideation_commands_cross_project::*;
pub use ideation_commands_dependencies::*;
pub use ideation_commands_export::*;
pub use ideation_commands_orchestrator::*;
pub use ideation_commands_proposals::*;
pub use ideation_commands_session::*;

// Re-export helper function for tests
pub use ideation_commands_dependencies::build_dependency_graph;

#[cfg(test)]
mod tests;
