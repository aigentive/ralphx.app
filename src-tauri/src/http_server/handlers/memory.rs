// Memory MCP tool handlers
// These handlers are restricted to memory agents only (memory-maintainer, memory-capture)
// Access control is enforced via three-layer allowlist model

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use tracing::{error, info};

use super::*;
use crate::domain::entities::{
    ArchiveJobPayload, ArchiveJobType,
    MemoryActorType, MemoryArchiveJob, MemoryBucket, MemoryEntry,
    MemoryEntryId, MemoryEvent, MemoryStatus, ProcessId,
};
use crate::domain::entities::types::ProjectId;
use crate::domain::services::RuleIngestionService;

// ============================================================================
// Handler: upsert_memories
// ============================================================================

pub async fn upsert_memories(
    State(state): State<HttpServerState>,
    Json(req): Json<UpsertMemoriesRequest>,
) -> Result<Json<UpsertMemoriesResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id.clone());
    let mut inserted = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for input in &req.memories {
        // Parse bucket
        let bucket = match input.bucket.parse::<MemoryBucket>() {
            Ok(b) => b,
            Err(_) => {
                error!("Invalid bucket: {}", input.bucket);
                failed += 1;
                continue;
            }
        };

        // Compute content hash for deduplication
        let content_hash =
            MemoryEntry::compute_content_hash(&input.title, &input.summary, &input.details_markdown);

        // Check for duplicate
        let existing = state
            .app_state
            .memory_entry_repo
            .find_by_content_hash(&project_id, &bucket, &content_hash)
            .await
            .map_err(|e| {
                error!("Failed to check content hash: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if existing.is_some() {
            skipped += 1;
            continue;
        }

        // Create new memory entry
        let mut entry = MemoryEntry::new(
            project_id.clone(),
            bucket,
            input.title.clone(),
            input.summary.clone(),
            input.details_markdown.clone(),
            input.scope_paths.clone(),
            content_hash,
        );
        entry.source_context_type = input.source_context_type.clone();
        entry.source_context_id = input.source_context_id.clone();
        entry.source_conversation_id = input.source_conversation_id.clone();
        entry.quality_score = input.quality_score;

        match state.app_state.memory_entry_repo.create(entry).await {
            Ok(_) => inserted += 1,
            Err(e) => {
                error!("Failed to create memory entry: {}", e);
                failed += 1;
            }
        }
    }

    info!(
        "upsert_memories: inserted={}, skipped={}, failed={}",
        inserted, skipped, failed
    );

    Ok(Json(UpsertMemoriesResponse {
        inserted,
        skipped,
        failed,
        message: format!(
            "Processed {} memories: {} inserted, {} skipped (duplicates), {} failed",
            req.memories.len(),
            inserted,
            skipped,
            failed
        ),
    }))
}

// ============================================================================
// Handler: mark_memory_obsolete
// ============================================================================

pub async fn mark_memory_obsolete(
    State(state): State<HttpServerState>,
    Json(req): Json<MarkMemoryObsoleteRequest>,
) -> Result<Json<MarkMemoryObsoleteResponse>, StatusCode> {
    let memory_id = MemoryEntryId::from_string(&req.memory_id);

    // Verify entry exists
    let entry = state
        .app_state
        .memory_entry_repo
        .get_by_id(&memory_id)
        .await
        .map_err(|e| {
            error!("Failed to get memory entry: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update status to obsolete
    state
        .app_state
        .memory_entry_repo
        .update_status(&memory_id, MemoryStatus::Obsolete)
        .await
        .map_err(|e| {
            error!("Failed to update memory status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Record audit event
    let event = MemoryEvent::new(
        ProcessId::from_string(entry.project_id.0.clone()),
        "memory_obsoleted",
        MemoryActorType::System,
        json!({
            "memory_id": req.memory_id,
            "title": entry.title,
        }),
    );
    let _ = state.app_state.memory_event_repo.create(event).await;

    Ok(Json(MarkMemoryObsoleteResponse {
        success: true,
        message: format!("Memory {} marked as obsolete", req.memory_id),
    }))
}

// ============================================================================
// Handler: refresh_memory_rule_index
// ============================================================================

pub async fn refresh_memory_rule_index(
    State(_state): State<HttpServerState>,
    Json(_req): Json<RefreshMemoryRuleIndexRequest>,
) -> Result<Json<RefreshMemoryRuleIndexResponse>, StatusCode> {
    // This handler requires memory_rule_bindings which is not yet part of the
    // repository layer. Return a stub response for now.
    Ok(Json(RefreshMemoryRuleIndexResponse {
        files_refreshed: 0,
        message: "Rule binding refresh not yet implemented - requires memory_rule_binding repository".to_string(),
    }))
}

// ============================================================================
// Handler: ingest_rule_file
// ============================================================================

pub async fn ingest_rule_file(
    State(state): State<HttpServerState>,
    Json(req): Json<IngestRuleFileRequest>,
) -> Result<Json<IngestRuleFileResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id.clone());

    let service = RuleIngestionService::new(
        Arc::clone(&state.app_state.memory_entry_repo),
        Arc::clone(&state.app_state.memory_event_repo),
        Arc::clone(&state.app_state.memory_archive_repo),
    );

    let result = service
        .ingest_rule_file(project_id, &req.rule_file_path)
        .await
        .map_err(|e| {
            error!("Failed to ingest rule file '{}': {}", req.rule_file_path, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(
        "ingest_rule_file '{}': created={}, updated={}, rewritten={}",
        req.rule_file_path, result.memories_created, result.memories_updated, result.file_rewritten
    );

    Ok(Json(IngestRuleFileResponse {
        memories_created: result.memories_created,
        memories_updated: result.memories_updated,
        file_rewritten: result.file_rewritten,
        message: format!(
            "Ingested '{}': {} memories created, {} updated",
            req.rule_file_path, result.memories_created, result.memories_updated
        ),
    }))
}

// ============================================================================
// Handler: rebuild_archive_snapshots
// ============================================================================

pub async fn rebuild_archive_snapshots(
    State(state): State<HttpServerState>,
    Json(req): Json<RebuildArchiveSnapshotsRequest>,
) -> Result<Json<RebuildArchiveSnapshotsResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id.clone());

    let payload = ArchiveJobPayload::full_rebuild(false);
    let job = MemoryArchiveJob::new(
        project_id,
        ArchiveJobType::FullRebuild,
        payload,
    );
    let job_id = job.id.to_string();

    state
        .app_state
        .memory_archive_repo
        .create(job)
        .await
        .map_err(|e| {
            error!("Failed to create archive rebuild job: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("rebuild_archive_snapshots: enqueued job {}", job_id);

    Ok(Json(RebuildArchiveSnapshotsResponse {
        job_id,
        message: "Full archive rebuild job enqueued".to_string(),
    }))
}
