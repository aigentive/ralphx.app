use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS solution_critique_gap_actions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id TEXT NOT NULL,
            critique_artifact_id TEXT NOT NULL,
            context_artifact_id TEXT NOT NULL,
            gap_id TEXT NOT NULL,
            gap_fingerprint TEXT NOT NULL,
            action TEXT NOT NULL CHECK (action IN ('promoted', 'deferred', 'covered', 'reopened')),
            note TEXT,
            actor_kind TEXT NOT NULL DEFAULT 'human',
            verification_generation INTEGER,
            promoted_round INTEGER,
            created_at TEXT NOT NULL,
            FOREIGN KEY(session_id) REFERENCES ideation_sessions(id)
        );

        CREATE INDEX IF NOT EXISTS idx_solution_critique_gap_actions_target
            ON solution_critique_gap_actions(session_id, target_type, target_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_solution_critique_gap_actions_gap
            ON solution_critique_gap_actions(critique_artifact_id, gap_id, created_at DESC);
        "#,
    )?;
    Ok(())
}
