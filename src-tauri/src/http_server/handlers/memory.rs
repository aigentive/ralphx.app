// Memory MCP tool handlers
// These handlers are restricted to memory agents only (memory-maintainer, memory-capture)
// Access control is enforced via three-layer allowlist model

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use tracing::warn;

use super::*;

// ============================================================================
// Handler: upsert_memories
// ============================================================================

pub async fn upsert_memories(
    State(_state): State<HttpServerState>,
    Json(_req): Json<UpsertMemoriesRequest>,
) -> Result<Json<UpsertMemoriesResponse>, StatusCode> {
    // TODO: Implement once memory_entry_repository is available
    // 1. Validate project_id exists
    // 2. For each memory entry:
    //    a. Compute content_hash from (title + summary + details_markdown)
    //    b. Check if hash already exists for this project+bucket
    //    c. If exists: skip (deduplication)
    //    d. If new: validate bucket + scope_paths, then insert
    // 3. Return counts: inserted, skipped, failed

    warn!("upsert_memories called but not yet implemented (awaiting repository layer)");

    Ok(Json(UpsertMemoriesResponse {
        inserted: 0,
        skipped: 0,
        failed: 0,
        message: "Not yet implemented - awaiting memory repository layer".to_string(),
    }))
}

// ============================================================================
// Handler: mark_memory_obsolete
// ============================================================================

pub async fn mark_memory_obsolete(
    State(_state): State<HttpServerState>,
    Json(_req): Json<MarkMemoryObsoleteRequest>,
) -> Result<Json<MarkMemoryObsoleteResponse>, StatusCode> {
    // TODO: Implement once memory_entry_repository is available
    // 1. Validate memory_id exists
    // 2. Update status field to 'obsolete' (soft delete)
    // 3. Record event in memory_events table
    // 4. Return success status

    warn!("mark_memory_obsolete called but not yet implemented (awaiting repository layer)");

    Ok(Json(MarkMemoryObsoleteResponse {
        success: false,
        message: "Not yet implemented - awaiting memory repository layer".to_string(),
    }))
}

// ============================================================================
// Handler: refresh_memory_rule_index
// ============================================================================

pub async fn refresh_memory_rule_index(
    State(_state): State<HttpServerState>,
    Json(_req): Json<RefreshMemoryRuleIndexRequest>,
) -> Result<Json<RefreshMemoryRuleIndexResponse>, StatusCode> {
    // TODO: Implement once memory repositories are available
    // 1. Load memory_rule_bindings for project
    // 2. For each scope_key:
    //    a. Query memory_entries matching scope_paths
    //    b. Generate canonical index file format (frontmatter + summaries + memory IDs)
    //    c. Write to .claude/rules/<rule_file_path>
    //    d. Update last_synced_at + last_content_hash
    // 3. Return count of refreshed files

    warn!("refresh_memory_rule_index called but not yet implemented (awaiting repository layer)");

    Ok(Json(RefreshMemoryRuleIndexResponse {
        files_refreshed: 0,
        message: "Not yet implemented - awaiting memory repository layer".to_string(),
    }))
}

// ============================================================================
// Handler: ingest_rule_file
// ============================================================================

pub async fn ingest_rule_file(
    State(_state): State<HttpServerState>,
    Json(_req): Json<IngestRuleFileRequest>,
) -> Result<Json<IngestRuleFileResponse>, StatusCode> {
    // TODO: Implement once memory repositories are available
    // 1. Read rule file from filesystem (req.rule_file_path)
    // 2. Parse frontmatter (extract paths: globs)
    // 3. Parse content into semantic chunks
    // 4. Classify each chunk into bucket (architecture_patterns, implementation_discoveries, operational_playbooks)
    // 5. For each chunk:
    //    a. Compute content_hash
    //    b. Upsert to memory_entries with source_rule_file metadata
    // 6. Rewrite rule file to canonical index format
    // 7. Enqueue archive jobs for affected memories
    // 8. Return ingestion stats

    warn!("ingest_rule_file called but not yet implemented (awaiting repository layer)");

    Ok(Json(IngestRuleFileResponse {
        memories_created: 0,
        memories_updated: 0,
        file_rewritten: false,
        message: "Not yet implemented - awaiting memory repository layer".to_string(),
    }))
}

// ============================================================================
// Handler: rebuild_archive_snapshots
// ============================================================================

pub async fn rebuild_archive_snapshots(
    State(_state): State<HttpServerState>,
    Json(_req): Json<RebuildArchiveSnapshotsRequest>,
) -> Result<Json<RebuildArchiveSnapshotsResponse>, StatusCode> {
    // TODO: Implement once memory repositories are available
    // 1. Enqueue full rebuild job in memory_archive_jobs table
    // 2. Job will be processed by background archive service
    // 3. Return job_id for tracking

    warn!("rebuild_archive_snapshots called but not yet implemented (awaiting repository layer)");

    Ok(Json(RebuildArchiveSnapshotsResponse {
        job_id: "pending".to_string(),
        message: "Not yet implemented - awaiting memory repository layer".to_string(),
    }))
}
