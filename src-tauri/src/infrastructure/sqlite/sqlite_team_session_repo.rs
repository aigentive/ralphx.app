// SQLite-based TeamSessionRepository implementation
// Uses rusqlite with mutex-protected connection for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::team::{TeamSession, TeamSessionId, TeammateSnapshot};
use crate::domain::repositories::TeamSessionRepository;
use crate::error::{AppError, AppResult};

fn parse_datetime(s: &str) -> DateTime<Utc> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&ndt);
    }
    Utc::now()
}

fn parse_teammates(json: &str) -> Vec<TeammateSnapshot> {
    serde_json::from_str(json).unwrap_or_default()
}

pub struct SqliteTeamSessionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTeamSessionRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TeamSessionRepository for SqliteTeamSessionRepository {
    async fn create(&self, session: TeamSession) -> AppResult<TeamSession> {
        let conn = self.conn.lock().await;
        let teammate_json =
            serde_json::to_string(&session.teammates).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO team_sessions (id, team_name, context_id, context_type, lead_name, phase, teammate_json, created_at, updated_at, disbanded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                session.id.as_str(),
                session.team_name,
                session.context_id,
                session.context_type,
                session.lead_name,
                session.phase,
                teammate_json,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.disbanded_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    async fn get_by_id(&self, id: &TeamSessionId) -> AppResult<Option<TeamSession>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, team_name, context_id, context_type, lead_name, phase, teammate_json, created_at, updated_at, disbanded_at
             FROM team_sessions WHERE id = ?1",
            [id.as_str()],
            |row| {
                let teammate_json: String = row.get("teammate_json")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;
                let disbanded_at_str: Option<String> = row.get("disbanded_at")?;

                Ok(TeamSession {
                    id: TeamSessionId::from_string(row.get::<_, String>("id")?),
                    team_name: row.get("team_name")?,
                    context_id: row.get("context_id")?,
                    context_type: row.get("context_type")?,
                    lead_name: row.get("lead_name")?,
                    phase: row.get("phase")?,
                    teammates: parse_teammates(&teammate_json),
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                    disbanded_at: disbanded_at_str.map(|s| parse_datetime(&s)),
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_context(
        &self,
        context_type: &str,
        context_id: &str,
    ) -> AppResult<Vec<TeamSession>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, team_name, context_id, context_type, lead_name, phase, teammate_json, created_at, updated_at, disbanded_at
                 FROM team_sessions WHERE context_type = ?1 AND context_id = ?2 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sessions = stmt
            .query_map([context_type, context_id], |row| {
                let teammate_json: String = row.get("teammate_json")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;
                let disbanded_at_str: Option<String> = row.get("disbanded_at")?;

                Ok(TeamSession {
                    id: TeamSessionId::from_string(row.get::<_, String>("id")?),
                    team_name: row.get("team_name")?,
                    context_id: row.get("context_id")?,
                    context_type: row.get("context_type")?,
                    lead_name: row.get("lead_name")?,
                    phase: row.get("phase")?,
                    teammates: parse_teammates(&teammate_json),
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                    disbanded_at: disbanded_at_str.map(|s| parse_datetime(&s)),
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    async fn get_active_for_context(
        &self,
        context_type: &str,
        context_id: &str,
    ) -> AppResult<Option<TeamSession>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, team_name, context_id, context_type, lead_name, phase, teammate_json, created_at, updated_at, disbanded_at
             FROM team_sessions
             WHERE context_type = ?1 AND context_id = ?2 AND disbanded_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
            [context_type, context_id],
            |row| {
                let teammate_json: String = row.get("teammate_json")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;
                let disbanded_at_str: Option<String> = row.get("disbanded_at")?;

                Ok(TeamSession {
                    id: TeamSessionId::from_string(row.get::<_, String>("id")?),
                    team_name: row.get("team_name")?,
                    context_id: row.get("context_id")?,
                    context_type: row.get("context_type")?,
                    lead_name: row.get("lead_name")?,
                    phase: row.get("phase")?,
                    teammates: parse_teammates(&teammate_json),
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                    disbanded_at: disbanded_at_str.map(|s| parse_datetime(&s)),
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn update_phase(&self, id: &TeamSessionId, phase: &str) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE team_sessions SET phase = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![phase, Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn update_teammates(
        &self,
        id: &TeamSessionId,
        teammates: &[TeammateSnapshot],
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let json = serde_json::to_string(teammates).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE team_sessions SET teammate_json = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![json, Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn set_disbanded(&self, id: &TeamSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE team_sessions SET disbanded_at = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![now, now, id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
