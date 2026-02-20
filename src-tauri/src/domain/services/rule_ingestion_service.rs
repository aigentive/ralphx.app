// Rule ingestion service for ingesting .claude/rules files into memory system

use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use crate::domain::entities::types::ProjectId;
use crate::domain::entities::{
    ArchiveJobPayload, ArchiveJobType, MemoryActorType, MemoryArchiveJob, MemoryEntry, MemoryEvent,
};
use crate::domain::repositories::{
    MemoryArchiveRepository, MemoryEntryRepository, MemoryEventRepository,
};
use crate::domain::services::{BucketClassifier, IndexRewriter, RuleParser};
use crate::error::AppResult;

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
    memory_archive_repo: Arc<dyn MemoryArchiveRepository>,
}

impl RuleIngestionService {
    /// Create a new rule ingestion service
    pub fn new(
        memory_entry_repo: Arc<dyn MemoryEntryRepository>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
        memory_archive_repo: Arc<dyn MemoryArchiveRepository>,
    ) -> Self {
        Self {
            memory_entry_repo,
            memory_event_repo,
            memory_archive_repo,
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
        project_id: ProjectId,
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
                content_hash,
            );

            // Set source metadata
            memory.source_rule_file = Some(rule_file_path.to_str().unwrap_or_default().to_string());

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

            index_rewriter.rewrite_rule_file(
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
        project_id: &ProjectId,
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
    async fn enqueue_archive_job(&self, project_id: &ProjectId, memory_id: &str) -> AppResult<()> {
        let payload = ArchiveJobPayload::memory_snapshot(memory_id);
        let job =
            MemoryArchiveJob::new(project_id.clone(), ArchiveJobType::MemorySnapshot, payload);

        self.memory_archive_repo.create(job).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "rule_ingestion_service_tests.rs"]
mod tests;
