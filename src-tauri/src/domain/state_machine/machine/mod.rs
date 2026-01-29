// TaskStateMachine - statig-based state machine for task lifecycle
// This implements the core 14-state task lifecycle with hierarchical superstates

mod transitions;
pub mod types;

// Re-export public types
pub use types::{ParseStateError, Response, State, TaskStateMachine};

#[cfg(test)]
mod tests;
