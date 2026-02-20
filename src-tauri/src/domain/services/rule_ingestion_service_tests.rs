use super::*;
use crate::domain::entities::types::ProjectId;
use crate::infrastructure::sqlite::{
    run_migrations, SqliteMemoryArchiveRepository, SqliteMemoryEntryRepository,
    SqliteMemoryEventRepository,
};
use rusqlite::Connection;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create test service with file-based database
async fn create_test_service() -> (RuleIngestionService, TempDir) {
    // Create temp dir for both test files and database
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create three connections to the same database file
    let conn1 = Connection::open(&db_path).unwrap();
    run_migrations(&conn1).unwrap();

    // Create test project in database
    conn1
        .execute(
            "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            ["test-project-123", "Test Project", "/tmp/test"],
        )
        .unwrap();

    let conn2 = Connection::open(&db_path).unwrap();
    let conn3 = Connection::open(&db_path).unwrap();

    // Create repositories with Arc wrapping
    let memory_entry_repo =
        Arc::new(SqliteMemoryEntryRepository::new(conn1)) as Arc<dyn MemoryEntryRepository>;

    let memory_event_repo =
        Arc::new(SqliteMemoryEventRepository::new(conn2)) as Arc<dyn MemoryEventRepository>;

    let memory_archive_repo =
        Arc::new(SqliteMemoryArchiveRepository::new(conn3)) as Arc<dyn MemoryArchiveRepository>;

    let service =
        RuleIngestionService::new(memory_entry_repo, memory_event_repo, memory_archive_repo);

    (service, temp_dir)
}

/// Helper to create test project
fn create_test_project() -> ProjectId {
    ProjectId::from_string("test-project-123".to_string())
}

#[tokio::test]
async fn test_ingest_new_rule_file() {
    let (service, temp_dir) = create_test_service().await;
    let project_id = create_test_project();

    // Create a test rule file
    let rule_content = r#"---
paths:
  - "src/domain/**"
  - "src/application/**"
---

# State Machine Pattern

State transitions must go through TransitionHandler.

## Details

The TransitionHandler ensures proper side effects are executed.
"#;

    let rule_path = temp_dir.path().join("state-machine.md");
    fs::write(&rule_path, rule_content).unwrap();

    // Ingest the file
    let result = service
        .ingest_rule_file(project_id.clone(), &rule_path)
        .await
        .unwrap();

    // Verify results (2 chunks: "State Machine Pattern" and "Details")
    assert!(result.memories_created >= 1);
    assert!(result.file_rewritten);

    // Verify file was rewritten
    let rewritten_content = fs::read_to_string(&rule_path).unwrap();
    assert!(rewritten_content.contains("# Memory Index:"));
    assert!(rewritten_content.contains("## Summary"));
    assert!(rewritten_content.contains("## Memory References"));
    assert!(rewritten_content.contains("## Retrieval"));
}

#[tokio::test]
async fn test_paths_preserved_in_index() {
    let (service, temp_dir) = create_test_service().await;
    let project_id = create_test_project();

    let rule_content = r#"---
paths:
  - "src/domain/**"
  - "src/application/**"
  - "src/api/**"
---

# Test Rule

Some content here.
"#;

    let rule_path = temp_dir.path().join("test-rule.md");
    fs::write(&rule_path, rule_content).unwrap();

    service
        .ingest_rule_file(project_id, &rule_path)
        .await
        .unwrap();

    // Verify paths are preserved and normalized in rewritten file
    let rewritten_content = fs::read_to_string(&rule_path).unwrap();
    assert!(rewritten_content.contains("paths:"));
    assert!(rewritten_content.contains("src/api/**"));
    assert!(rewritten_content.contains("src/application/**"));
    assert!(rewritten_content.contains("src/domain/**"));
}

#[tokio::test]
async fn test_re_ingest_is_idempotent() {
    let (service, temp_dir) = create_test_service().await;
    let project_id = create_test_project();

    let rule_content = r#"---
paths:
  - "src/**"
---

# Test Rule

Content for testing idempotency.
"#;

    let rule_path = temp_dir.path().join("idempotent-test.md");
    fs::write(&rule_path, rule_content).unwrap();

    // First ingestion
    let result1 = service
        .ingest_rule_file(project_id.clone(), &rule_path)
        .await
        .unwrap();

    let first_created = result1.memories_created;
    assert!(first_created >= 1);
    assert_eq!(result1.memories_skipped, 0);

    // Re-ingest the same content (should skip duplicates)
    // First, restore the original content since it was rewritten
    fs::write(&rule_path, rule_content).unwrap();

    let result2 = service
        .ingest_rule_file(project_id, &rule_path)
        .await
        .unwrap();

    assert_eq!(result2.memories_created, 0);
    assert_eq!(result2.memories_skipped, first_created); // All duplicates detected
    assert!(!result2.file_rewritten); // No new memories, so no rewrite
}

#[tokio::test]
async fn test_multiple_chunks_ingested() {
    let (service, temp_dir) = create_test_service().await;
    let project_id = create_test_project();

    let rule_content = r#"---
paths:
  - "src/**"
---

# First Pattern

Description of first pattern.

# Second Discovery

Details about second discovery.

# Third Playbook

Operational procedure details.
"#;

    let rule_path = temp_dir.path().join("multi-chunk.md");
    fs::write(&rule_path, rule_content).unwrap();

    let result = service
        .ingest_rule_file(project_id, &rule_path)
        .await
        .unwrap();

    // Should create 3-4 memory entries (one per H1/H2 heading)
    assert!(result.memories_created >= 3);
    assert!(result.file_rewritten);

    // Verify index contains all three memories
    let rewritten_content = fs::read_to_string(&rule_path).unwrap();
    assert!(rewritten_content.contains("First Pattern"));
    assert!(rewritten_content.contains("Second Discovery"));
    assert!(rewritten_content.contains("Third Playbook"));
}
