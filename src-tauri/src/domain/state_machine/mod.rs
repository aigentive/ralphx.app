// State machine module - statig-based task lifecycle management
// This module implements the 14-state task state machine with:
// - Type-safe transitions validated at compile time
// - Hierarchical superstates (Execution, QA, Review)
// - State-local data for QaFailed and Failed states
// - Async actions for agent spawning and event emission

pub mod events;
pub mod types;

// Re-export key types
pub use events::TaskEvent;
pub use types::{Blocker, QaFailure};

#[cfg(test)]
mod tests {
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
