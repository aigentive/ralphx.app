use crate::error::{AppError, AppResult};
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS design_systems (
            id TEXT PRIMARY KEY,
            primary_project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            current_schema_version_id TEXT,
            storage_root_ref TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            archived_at DATETIME
        );

        CREATE TABLE IF NOT EXISTS design_system_sources (
            id TEXT PRIMARY KEY,
            design_system_id TEXT NOT NULL REFERENCES design_systems(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            selected_paths_json TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            git_commit TEXT,
            source_hashes_json TEXT NOT NULL,
            last_analyzed_at DATETIME
        );

        CREATE TABLE IF NOT EXISTS design_schema_versions (
            id TEXT PRIMARY KEY,
            design_system_id TEXT NOT NULL REFERENCES design_systems(id) ON DELETE CASCADE,
            version TEXT NOT NULL,
            schema_artifact_id TEXT NOT NULL,
            manifest_artifact_id TEXT NOT NULL,
            styleguide_artifact_id TEXT NOT NULL,
            status TEXT NOT NULL,
            created_by_run_id TEXT,
            created_at DATETIME NOT NULL,
            UNIQUE(design_system_id, version)
        );

        CREATE TABLE IF NOT EXISTS design_styleguide_items (
            id TEXT PRIMARY KEY,
            design_system_id TEXT NOT NULL REFERENCES design_systems(id) ON DELETE CASCADE,
            schema_version_id TEXT NOT NULL REFERENCES design_schema_versions(id) ON DELETE CASCADE,
            item_id TEXT NOT NULL,
            group_name TEXT NOT NULL,
            label TEXT NOT NULL,
            summary TEXT NOT NULL,
            preview_artifact_id TEXT,
            source_refs_json TEXT NOT NULL,
            confidence TEXT NOT NULL,
            approval_status TEXT NOT NULL,
            feedback_status TEXT NOT NULL,
            updated_at DATETIME NOT NULL,
            UNIQUE(schema_version_id, item_id)
        );

        CREATE TABLE IF NOT EXISTS design_styleguide_feedback (
            id TEXT PRIMARY KEY,
            design_system_id TEXT NOT NULL REFERENCES design_systems(id) ON DELETE CASCADE,
            schema_version_id TEXT NOT NULL REFERENCES design_schema_versions(id) ON DELETE CASCADE,
            item_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
            message_id TEXT REFERENCES chat_messages(id) ON DELETE SET NULL,
            preview_artifact_id TEXT,
            source_refs_json TEXT NOT NULL,
            feedback TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            resolved_at DATETIME
        );

        CREATE TABLE IF NOT EXISTS design_runs (
            id TEXT PRIMARY KEY,
            design_system_id TEXT NOT NULL REFERENCES design_systems(id) ON DELETE CASCADE,
            conversation_id TEXT REFERENCES chat_conversations(id) ON DELETE SET NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            input_summary TEXT NOT NULL,
            output_artifact_ids_json TEXT NOT NULL,
            started_at DATETIME,
            completed_at DATETIME,
            error TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_design_systems_project_active
            ON design_systems(primary_project_id, archived_at, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_design_system_sources_system
            ON design_system_sources(design_system_id, role);

        CREATE INDEX IF NOT EXISTS idx_design_schema_versions_system
            ON design_schema_versions(design_system_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_design_styleguide_items_system_schema
            ON design_styleguide_items(design_system_id, schema_version_id);

        CREATE INDEX IF NOT EXISTS idx_design_styleguide_feedback_open
            ON design_styleguide_feedback(design_system_id, status, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_design_runs_system
            ON design_runs(design_system_id, started_at DESC, completed_at DESC);",
    )
    .map_err(|error| AppError::Database(error.to_string()))?;

    Ok(())
}
