use super::*;

use super::*;

#[test]
fn test_classify_architecture_pattern() {
    let title = "State Machine Design";
    let content = "The task state machine handles transitions between states. Each state has specific invariants and constraints.";

    let bucket = BucketClassifier::classify(title, content);
    assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
}

#[test]
fn test_classify_implementation_discovery() {
    let title = "Async Bug Fix";
    let content = "Found a gotcha with tokio async runtime. The issue was caused by a breaking change in the library upgrade.";

    let bucket = BucketClassifier::classify(title, content);
    assert_eq!(bucket, MemoryBucket::ImplementationDiscoveries);
}

#[test]
fn test_classify_operational_playbook() {
    let title = "Deployment Procedure";
    let content = "Step-by-step guide for deployment: 1. Backup database 2. Run migrations 3. Deploy new version 4. Verify health checks.";

    let bucket = BucketClassifier::classify(title, content);
    assert_eq!(bucket, MemoryBucket::OperationalPlaybooks);
}

#[test]
fn test_default_to_architecture() {
    let title = "Random Content";
    let content = "This is some random content without specific keywords.";

    let bucket = BucketClassifier::classify(title, content);
    // Should default to architecture patterns
    assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
}

#[test]
fn test_case_insensitive() {
    let title = "STATE MACHINE";
    let content = "THE STATE MACHINE HANDLES TRANSITIONS.";

    let bucket = BucketClassifier::classify(title, content);
    assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
}
