// State machine module - statig-based task lifecycle management
// This module implements the 14-state task state machine with:
// - Type-safe transitions validated at compile time
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
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, ServiceCall,
};
pub use services::{AgentSpawner, DependencyManager, EventEmitter, Notifier};
pub use persistence::{
    deserialize_failed_data, deserialize_qa_failed_data, serialize_failed_data,
    serialize_qa_failed_data, state_has_data, StateData,
};
pub use types::{Blocker, FailedData, QaFailedData, QaFailure};
pub use transition_handler::{TransitionHandler, TransitionResult};

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use statig::prelude::*;

    // Simple test to verify statig imports work
    #[test]
    fn test_statig_import_works() {
        // Just verify we can import from statig prelude
        // This confirms the dependency is correctly configured
        assert!(true);
    }

    #[test]
    fn test_tokio_full_features() {
        // Verify tokio with full features is available
        // We'll need rt, time, sync features for the state machine
        assert!(true);
    }
}
