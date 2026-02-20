use super::*;
use crate::domain::entities::{MemoryBucket, MemoryEntry, ProjectId};

#[test]
fn test_normalize_paths() {
    let rewriter = IndexRewriter::new();

    let paths = vec![
        "src/components/**".to_string(),
        "src/api/**".to_string(),
        "src/components/**".to_string(), // duplicate
        "src-tauri/**".to_string(),
    ];

    let normalized = rewriter.normalize_paths(paths);

    assert_eq!(normalized.len(), 3); // duplicates removed
    assert_eq!(normalized[0], "src-tauri/**"); // sorted alphabetically
    assert_eq!(normalized[1], "src/api/**");
    assert_eq!(normalized[2], "src/components/**");
}

#[test]
fn test_derive_title_from_path() {
    let rewriter = IndexRewriter::new();

    assert_eq!(
        rewriter.derive_title_from_path(".claude/rules/task-state-machine.md"),
        "Task State Machine"
    );

    assert_eq!(
        rewriter.derive_title_from_path(".claude/rules/api_layer.md"),
        "Api Layer"
    );

    assert_eq!(
        rewriter.derive_title_from_path("stream-features.md"),
        "Stream Features"
    );
}

#[test]
fn test_generate_index_content_empty() {
    let rewriter = IndexRewriter::new();

    let paths = vec!["src/**".to_string()];
    let memories: Vec<MemoryEntry> = vec![];

    let content = rewriter
        .generate_index_content("test.md", paths, &memories)
        .unwrap();

    assert!(content.contains("---\npaths:\n  - \"src/**\"\n---"));
    assert!(content.contains("## Summary\n\n(No memory entries ingested)"));
    assert!(content.contains("## Memory References\n\n(No memory entries)"));
    assert!(content.contains("## Retrieval"));
}

#[test]
fn test_generate_index_content_with_memories() {
    let rewriter = IndexRewriter::new();

    let project_id = ProjectId::from_string("test-project".to_string());
    let memory1 = MemoryEntry::new(
        project_id.clone(),
        MemoryBucket::ArchitecturePatterns,
        "State Machine Pattern".to_string(),
        "State transitions must go through TransitionHandler".to_string(),
        "Details about state machine...".to_string(),
        vec!["src/domain/**".to_string()],
        MemoryEntry::compute_content_hash(
            "State Machine Pattern",
            "State transitions must go through TransitionHandler",
            "Details about state machine...",
        ),
    );

    let memory2 = MemoryEntry::new(
        project_id,
        MemoryBucket::ImplementationDiscoveries,
        "Async Trait Gotcha".to_string(),
        "async_trait macro required for async methods in traits".to_string(),
        "Details about async traits...".to_string(),
        vec!["src/**".to_string()],
        MemoryEntry::compute_content_hash(
            "Async Trait Gotcha",
            "async_trait macro required for async methods in traits",
            "Details about async traits...",
        ),
    );

    let paths = vec!["src/**".to_string()];
    let memories = vec![memory1, memory2];

    let content = rewriter
        .generate_index_content("test.md", paths, &memories)
        .unwrap();

    assert!(content.contains("State transitions must go through TransitionHandler"));
    assert!(content.contains("async_trait macro required"));
    assert!(content.contains("### architecture_patterns"));
    assert!(content.contains("### implementation_discoveries"));
    assert!(content.contains("`get_memories_for_paths`"));
}

#[test]
fn test_group_memories_by_bucket() {
    let rewriter = IndexRewriter::new();

    let project_id = ProjectId::from_string("test-project".to_string());
    let memory1 = MemoryEntry::new(
        project_id.clone(),
        MemoryBucket::ArchitecturePatterns,
        "Pattern 1".to_string(),
        "Summary 1".to_string(),
        "Details 1".to_string(),
        vec![],
        MemoryEntry::compute_content_hash("Pattern 1", "Summary 1", "Details 1"),
    );

    let memory2 = MemoryEntry::new(
        project_id.clone(),
        MemoryBucket::ArchitecturePatterns,
        "Pattern 2".to_string(),
        "Summary 2".to_string(),
        "Details 2".to_string(),
        vec![],
        MemoryEntry::compute_content_hash("Pattern 2", "Summary 2", "Details 2"),
    );

    let memory3 = MemoryEntry::new(
        project_id,
        MemoryBucket::ImplementationDiscoveries,
        "Discovery 1".to_string(),
        "Summary 3".to_string(),
        "Details 3".to_string(),
        vec![],
        MemoryEntry::compute_content_hash("Discovery 1", "Summary 3", "Details 3"),
    );

    let memories = vec![memory1, memory2, memory3];
    let grouped = rewriter.group_memories_by_bucket(&memories);

    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].0, "architecture_patterns");
    assert_eq!(grouped[0].1.len(), 2);
    assert_eq!(grouped[1].0, "implementation_discoveries");
    assert_eq!(grouped[1].1.len(), 1);
}
