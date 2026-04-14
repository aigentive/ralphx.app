use crate::error::{AppError, AppResult};
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "verification_current_round",
        "INTEGER",
    )?;
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "verification_max_rounds",
        "INTEGER",
    )?;
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "verification_gap_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "verification_gap_score",
        "INTEGER",
    )?;
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "verification_convergence_reason",
        "TEXT",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS verification_runs (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            generation INTEGER NOT NULL,
            status TEXT NOT NULL,
            in_progress INTEGER NOT NULL DEFAULT 0,
            current_round INTEGER,
            max_rounds INTEGER,
            best_round_index INTEGER,
            convergence_reason TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            completed_at DATETIME,
            UNIQUE(session_id, generation)
        );

        CREATE TABLE IF NOT EXISTS verification_rounds (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES verification_runs(id) ON DELETE CASCADE,
            round_number INTEGER NOT NULL,
            gap_score INTEGER NOT NULL DEFAULT 0,
            parse_failed INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(run_id, round_number)
        );

        CREATE TABLE IF NOT EXISTS verification_round_gaps (
            id TEXT PRIMARY KEY,
            round_id TEXT NOT NULL REFERENCES verification_rounds(id) ON DELETE CASCADE,
            sort_order INTEGER NOT NULL,
            severity TEXT NOT NULL,
            category TEXT NOT NULL,
            description TEXT NOT NULL,
            why_it_matters TEXT,
            source TEXT,
            fingerprint TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS verification_run_current_gaps (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES verification_runs(id) ON DELETE CASCADE,
            sort_order INTEGER NOT NULL,
            severity TEXT NOT NULL,
            category TEXT NOT NULL,
            description TEXT NOT NULL,
            why_it_matters TEXT,
            source TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_verification_runs_session_generation
            ON verification_runs(session_id, generation DESC);

        CREATE INDEX IF NOT EXISTS idx_verification_runs_status
            ON verification_runs(status, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_verification_rounds_run
            ON verification_rounds(run_id, round_number ASC);

        CREATE INDEX IF NOT EXISTS idx_verification_round_gaps_round
            ON verification_round_gaps(round_id, sort_order ASC);

        CREATE INDEX IF NOT EXISTS idx_verification_run_current_gaps_run
            ON verification_run_current_gaps(run_id, sort_order ASC);",
    )
    .map_err(|error| AppError::Database(error.to_string()))?;

    Ok(())
}
