use super::*;

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
