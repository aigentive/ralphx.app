// Migration v28 (schema v29): Repair schema drift for databases with skipped/misaligned versions.
//
// This migration is intentionally idempotent. It recreates the Memory Framework V2
// tables/indexes if they are missing, even when older migration versions were already
// marked as applied in schema_migrations due to branch/version drift.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v28: Repair missing memory framework tables/indexes.
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "-- Project-level memory settings
        CREATE TABLE IF NOT EXISTS project_memory_settings (
            project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            enabled INTEGER NOT NULL DEFAULT 1,
            maintenance_categories_json TEXT NOT NULL DEFAULT '[\"execution\",\"review\",\"merge\"]',
            capture_categories_json TEXT NOT NULL DEFAULT '[\"planning\",\"execution\",\"review\"]',
            archive_enabled INTEGER NOT NULL DEFAULT 1,
            archive_path TEXT NOT NULL DEFAULT '.claude/memory-archive',
            archive_auto_commit INTEGER NOT NULL DEFAULT 0,
            retain_rule_snapshots INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );

        -- Canonical memory entries with bucket taxonomy
        CREATE TABLE IF NOT EXISTS memory_entries (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            bucket TEXT NOT NULL CHECK (bucket IN ('architecture_patterns', 'implementation_discoveries', 'operational_playbooks')),
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            details_markdown TEXT NOT NULL,
            scope_paths_json TEXT NOT NULL DEFAULT '[]',
            source_context_type TEXT,
            source_context_id TEXT,
            source_conversation_id TEXT,
            source_rule_file TEXT,
            quality_score REAL,
            status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'obsolete', 'archived')),
            content_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );

        CREATE INDEX IF NOT EXISTS idx_memory_entries_project_bucket_status
            ON memory_entries(project_id, bucket, status);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_conversation
            ON memory_entries(project_id, source_conversation_id);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_content_hash
            ON memory_entries(content_hash);

        -- Audit trail for memory operations
        CREATE TABLE IF NOT EXISTS memory_events (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            event_type TEXT NOT NULL,
            actor_type TEXT NOT NULL CHECK (actor_type IN ('system', 'memory-maintainer', 'memory-capture')),
            details_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );

        CREATE INDEX IF NOT EXISTS idx_memory_events_project
            ON memory_events(project_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_events_type
            ON memory_events(event_type, created_at DESC);

        -- Rule file sync state tracking
        CREATE TABLE IF NOT EXISTS memory_rule_bindings (
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            scope_key TEXT NOT NULL,
            rule_file_path TEXT NOT NULL,
            paths_json TEXT NOT NULL DEFAULT '[]',
            last_synced_at TEXT,
            last_content_hash TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            PRIMARY KEY (project_id, scope_key)
        );

        CREATE INDEX IF NOT EXISTS idx_memory_rule_bindings_file
            ON memory_rule_bindings(rule_file_path);

        -- Background job queue for archive snapshots
        CREATE TABLE IF NOT EXISTS memory_archive_jobs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            job_type TEXT NOT NULL CHECK (job_type IN ('memory_snapshot', 'rule_snapshot', 'full_rebuild')),
            payload_json TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'done', 'failed')),
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            started_at TEXT,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_memory_archive_jobs_project_status
            ON memory_archive_jobs(project_id, status);
        CREATE INDEX IF NOT EXISTS idx_memory_archive_jobs_status
            ON memory_archive_jobs(status, created_at);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Ensure all existing projects have default memory settings.
    conn.execute(
        "INSERT OR IGNORE INTO project_memory_settings (project_id)
         SELECT id FROM projects",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Repair older default merge strategy values if migration v27 was skipped/misaligned.
    conn.execute(
        "UPDATE projects SET merge_strategy = 'rebase_squash' WHERE merge_strategy = 'rebase';",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

