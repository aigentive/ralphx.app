// State machine module - statig-based task lifecycle management
// This module implements the 24-state task state machine with:
// - Runtime-enforced transitions with compile-time exhaustive match checking
// - Hierarchical superstates (Execution, QA, Review)
// - State-local data for QaFailed and Failed states
// - Async actions for agent spawning and event emission

pub mod context;
pub mod events;
pub mod machine;
pub mod mocks;
pub mod persistence;
pub mod services;
pub mod transition_handler;
pub mod types;

// Re-export key types
pub use context::{TaskContext, TaskServices};
pub use events::TaskEvent;
pub use machine::{ParseStateError, Response, State, TaskStateMachine};
pub use mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockTaskScheduler,
    ServiceCall,
};
pub use persistence::{
    deserialize_failed_data, deserialize_qa_failed_data, serialize_failed_data,
    serialize_qa_failed_data, state_has_data, StateData,
};
pub use services::{AgentSpawner, DependencyManager, EventEmitter, Notifier, TaskScheduler};
pub use transition_handler::{resolve_merge_branches, TransitionHandler, TransitionResult};
pub use types::{Blocker, FailedData, QaFailedData, QaFailure};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
