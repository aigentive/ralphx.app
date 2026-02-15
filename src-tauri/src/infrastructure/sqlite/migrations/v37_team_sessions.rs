// Migration v37: Team sessions and messages tables
//
// Creates persistence tables for agent team state recovery:
// - team_sessions: lead ID, teammate composition, phase
// - team_messages: sender, recipient, content, timestamp

use crate::error::{AppError, AppResult};
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Team sessions table — one row per active/historical team
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS team_sessions (
            id TEXT PRIMARY KEY,
            team_name TEXT NOT NULL,
            context_id TEXT NOT NULL,
            context_type TEXT NOT NULL,
            lead_name TEXT,
            phase TEXT NOT NULL DEFAULT 'forming',
            teammate_json TEXT NOT NULL DEFAULT '[]',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            disbanded_at DATETIME
        );

        CREATE INDEX IF NOT EXISTS idx_team_sessions_context
            ON team_sessions(context_type, context_id);

        CREATE INDEX IF NOT EXISTS idx_team_sessions_phase
            ON team_sessions(phase);

        CREATE TABLE IF NOT EXISTS team_messages (
            id TEXT PRIMARY KEY,
            team_session_id TEXT NOT NULL REFERENCES team_sessions(id),
            sender TEXT NOT NULL,
            recipient TEXT,
            content TEXT NOT NULL,
            message_type TEXT NOT NULL DEFAULT 'teammate_message',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_team_messages_session
            ON team_messages(team_session_id, created_at);

        CREATE INDEX IF NOT EXISTS idx_team_messages_sender
            ON team_messages(team_session_id, sender);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Seed the team-findings bucket (5th system bucket)
    conn.execute(
        "INSERT OR IGNORE INTO artifact_buckets (id, name, config_json, is_system)
         VALUES (?1, ?2, ?3, 1)",
        rusqlite::params![
            "team-findings",
            "Team Findings",
            r#"{"accepted_types":["team_research","team_analysis","team_summary"],"writers":["team-lead","system"],"readers":["all"]}"#,
        ],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
