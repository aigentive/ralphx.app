// Rule ingestion service for ingesting .claude/rules files into memory system

use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use crate::domain::entities::{
    MemoryActorType, MemoryArchiveJob, MemoryArchiveJobType, MemoryEntry, MemoryEvent, ProcessId,
};
use crate::domain::repositories::{
    MemoryArchiveJobRepository, MemoryEntryRepository, MemoryEventRepository,
};
use crate::domain::services::{BucketClassifier, IndexRewriter, RuleParser};
use crate::error::{AppResult};

/// Result of rule file ingestion
#[derive(Debug, Clone)]
pub struct IngestionResult {
    pub memories_created: usize,
    pub memories_updated: usize,
    pub memories_skipped: usize,
    pub file_rewritten: bool,
}

/// Service for ingesting rule files into the memory system
pub struct RuleIngestionService {
    memory_entry_repo: Arc<dyn MemoryEntryRepository>,
    memory_event_repo: Arc<dyn MemoryEventRepository>,
    memory_archive_job_repo: Arc<dyn MemoryArchiveJobRepository>,
}

impl RuleIngestionService {
    /// Create a new rule ingestion service
    pub fn new(
        memory_entry_repo: Arc<dyn MemoryEntryRepository>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
        memory_archive_job_repo: Arc<dyn MemoryArchiveJobRepository>,
    ) -> Self {
        Self {
            memory_entry_repo,
            memory_event_repo,
            memory_archive_job_repo,
        }
    }

    /// Ingest a rule file into the memory system
    ///
    /// This orchestrates the full ingestion pipeline:
    /// 1. Parse rule file (frontmatter + chunks)
    /// 2. Classify each chunk into a bucket
    /// 3. Upsert chunks as memory entries (deduplication by content hash)
    /// 4. Emit memory events for each action
    /// 5. Rewrite rule file to canonical index format
    /// 6. Enqueue archive jobs for affected memories
    pub async fn ingest_rule_file(
        &self,
        project_id: ProcessId,
        rule_file_path: impl AsRef<Path>,
    ) -> AppResult<IngestionResult> {
        let rule_file_path = rule_file_path.as_ref();

        // Parse rule file
        let parsed = RuleParser::parse_file(rule_file_path)?;

        let mut created = 0;
        let updated = 0;
        let mut skipped = 0;
        let mut ingested_memories = Vec::new();

        // Process each chunk
        for chunk in &parsed.chunks {
            // Classify chunk into bucket
            let bucket = BucketClassifier::classify(&chunk.title, &chunk.content);

            // Create memory entry
            let title = chunk.title.clone();
            let summary = Self::generate_summary(&chunk.content);
            let details = chunk.content.clone();

            // Compute content hash
            let content_hash = MemoryEntry::compute_content_hash(&title, &summary, &details);

            // Check for duplicate
            let existing = self
                .memory_entry_repo
                .find_by_content_hash(&project_id, &bucket, &content_hash)
                .await?;

            if existing.is_some() {
                // Duplicate found, skip
                skipped += 1;

                // Emit skip event
                self.emit_event(
                    &project_id,
                    "chunk_skipped",
                    json!({
                        "title": title,
                        "reason": "duplicate",
                        "content_hash": content_hash,
                    }),
                )
                .await?;

                continue;
            }

            // Create new memory entry
            let mut memory = MemoryEntry::new(
                project_id.clone(),
                bucket,
                title.clone(),
                summary,
                details,
                parsed.frontmatter.paths.clone(),
            );

            // Set source metadata
            memory.source_rule_file = Some(
                rule_file_path
                    .to_str()
                    .unwrap_or_default()
                    .to_string(),
            );

            // Insert into database
            let created_memory = self.memory_entry_repo.create(memory.clone()).await?;
            created += 1;

            // Track ingested memory for index rewrite
            ingested_memories.push(created_memory);

            // Emit creation event
            self.emit_event(
                &project_id,
                "memory_created",
                json!({
                    "memory_id": memory.id.as_str(),
                    "title": title,
                    "bucket": bucket.to_string(),
                }),
            )
            .await?;

            // Enqueue archive job for this memory
            self.enqueue_archive_job(&project_id, &memory.id.to_string())
                .await?;
        }

        // Rewrite rule file to canonical index format
        let file_rewritten = if !ingested_memories.is_empty() {
            let index_rewriter = IndexRewriter::new();
            let rule_file_path_str = rule_file_path.to_str().unwrap_or_default();

            index_rewriter
                .rewrite_rule_file(
                    rule_file_path_str,
                    parsed.frontmatter.paths.clone(),
                    &ingested_memories,
                )?;

            true
        } else {
            false
        };

        // Emit ingestion complete event
        self.emit_event(
            &project_id,
            "file_ingested",
            json!({
                "file_path": rule_file_path.to_str().unwrap_or_default(),
                "memories_created": created,
                "memories_skipped": skipped,
            }),
        )
        .await?;

        Ok(IngestionResult {
            memories_created: created,
            memories_updated: updated,
            memories_skipped: skipped,
            file_rewritten,
        })
    }

    /// Generate a summary from chunk content (first 2-3 sentences)
    fn generate_summary(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut summary_lines = Vec::new();
        let mut sentence_count = 0;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            summary_lines.push(trimmed);
            sentence_count += trimmed.matches(". ").count() + 1;

            if sentence_count >= 3 {
                break;
            }
        }

        summary_lines.join(" ")
    }

    /// Emit a memory event
    async fn emit_event(
        &self,
        project_id: &ProcessId,
        event_type: &str,
        details: serde_json::Value,
    ) -> AppResult<()> {
        let event = MemoryEvent::new(
            project_id.clone(),
            event_type,
            MemoryActorType::MemoryMaintainer,
            details,
        );

        self.memory_event_repo.create(event).await?;
        Ok(())
    }

    /// Enqueue an archive job for a memory
    async fn enqueue_archive_job(&self, project_id: &ProcessId, memory_id: &str) -> AppResult<()> {
        let job = MemoryArchiveJob::new(
            project_id.clone(),
            MemoryArchiveJobType::MemorySnapshot,
            json!({ "memory_id": memory_id }),
        );

        self.memory_archive_job_repo.create(job).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{
        run_migrations, SqliteMemoryArchiveJobRepository,
        SqliteMemoryEntryRepository, SqliteMemoryEventRepository,
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
        let memory_entry_repo = Arc::new(SqliteMemoryEntryRepository::new(conn1))
            as Arc<dyn MemoryEntryRepository>;

        let memory_event_repo = Arc::new(SqliteMemoryEventRepository::new(conn2))
            as Arc<dyn MemoryEventRepository>;

        let memory_archive_job_repo = Arc::new(SqliteMemoryArchiveJobRepository::new(conn3))
            as Arc<dyn MemoryArchiveJobRepository>;

        let service = RuleIngestionService::new(
            memory_entry_repo,
            memory_event_repo,
            memory_archive_job_repo,
        );

        (service, temp_dir)
    }

    /// Helper to create test project
    fn create_test_project() -> ProcessId {
        ProcessId::from_string("test-project-123")
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
}
