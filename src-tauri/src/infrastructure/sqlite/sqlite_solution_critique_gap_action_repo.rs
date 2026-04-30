use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use tokio::sync::Mutex;

use crate::domain::entities::{
    ArtifactId, ContextTargetRef, ContextTargetType, IdeationSessionId, SolutionCritiqueGapAction,
    SolutionCritiqueGapActionKind,
};
use crate::domain::repositories::SolutionCritiqueGapActionRepository;
use crate::error::{AppError, AppResult};

use super::DbConnection;

pub struct SqliteSolutionCritiqueGapActionRepository {
    db: DbConnection,
}

impl SqliteSolutionCritiqueGapActionRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }

    fn row_to_action(row: &Row<'_>) -> AppResult<SolutionCritiqueGapAction> {
        let target_type: String = row.get(3)?;
        let action: String = row.get(9)?;
        let created_at: String = row.get(14)?;
        Ok(SolutionCritiqueGapAction {
            id: row.get(0)?,
            session_id: row.get(1)?,
            project_id: row.get(2)?,
            target_type: target_type_from_db(&target_type)?,
            target_id: row.get(4)?,
            critique_artifact_id: row.get(5)?,
            context_artifact_id: row.get(6)?,
            gap_id: row.get(7)?,
            gap_fingerprint: row.get(8)?,
            action: action_from_db(&action)?,
            note: row.get(10)?,
            actor_kind: row.get(11)?,
            verification_generation: row.get(12)?,
            promoted_round: row.get::<_, Option<i64>>(13)?.map(|value| value as u32),
            created_at: parse_datetime(&created_at)?,
        })
    }
}

#[async_trait]
impl SolutionCritiqueGapActionRepository for SqliteSolutionCritiqueGapActionRepository {
    async fn append(&self, action: SolutionCritiqueGapAction) -> AppResult<()> {
        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO solution_critique_gap_actions (
                        id, session_id, project_id, target_type, target_id,
                        critique_artifact_id, context_artifact_id, gap_id, gap_fingerprint,
                        action, note, actor_kind, verification_generation, promoted_round,
                        created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        action.id,
                        action.session_id,
                        action.project_id,
                        target_type_to_db(action.target_type),
                        action.target_id,
                        action.critique_artifact_id,
                        action.context_artifact_id,
                        action.gap_id,
                        action.gap_fingerprint,
                        action_to_db(action.action),
                        action.note,
                        action.actor_kind,
                        action.verification_generation,
                        action.promoted_round.map(|value| value as i64),
                        action.created_at.to_rfc3339(),
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn list_for_critique(
        &self,
        critique_artifact_id: &ArtifactId,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>> {
        let critique_artifact_id = critique_artifact_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, project_id, target_type, target_id,
                            critique_artifact_id, context_artifact_id, gap_id, gap_fingerprint,
                            action, note, actor_kind, verification_generation, promoted_round,
                            created_at
                     FROM solution_critique_gap_actions
                     WHERE critique_artifact_id = ?1
                     ORDER BY created_at DESC, id DESC",
                )?;
                let mut rows = stmt.query(params![critique_artifact_id])?;
                let mut actions = Vec::new();
                while let Some(row) = rows.next()? {
                    actions.push(Self::row_to_action(row)?);
                }
                Ok(actions)
            })
            .await
    }

    async fn list_for_target(
        &self,
        session_id: &IdeationSessionId,
        target: &ContextTargetRef,
    ) -> AppResult<Vec<SolutionCritiqueGapAction>> {
        let session_id = session_id.as_str().to_string();
        let target_type = target_type_to_db(target.target_type);
        let target_id = target.id.clone();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, project_id, target_type, target_id,
                            critique_artifact_id, context_artifact_id, gap_id, gap_fingerprint,
                            action, note, actor_kind, verification_generation, promoted_round,
                            created_at
                     FROM solution_critique_gap_actions
                     WHERE session_id = ?1 AND target_type = ?2 AND target_id = ?3
                     ORDER BY created_at DESC, id DESC",
                )?;
                let mut rows = stmt.query(params![session_id, target_type, target_id])?;
                let mut actions = Vec::new();
                while let Some(row) = rows.next()? {
                    actions.push(Self::row_to_action(row)?);
                }
                Ok(actions)
            })
            .await
    }
}

fn target_type_to_db(target_type: ContextTargetType) -> &'static str {
    match target_type {
        ContextTargetType::PlanArtifact => "plan_artifact",
        ContextTargetType::Artifact => "artifact",
        ContextTargetType::ChatMessage => "chat_message",
        ContextTargetType::AgentRun => "agent_run",
        ContextTargetType::Task => "task",
        ContextTargetType::TaskExecution => "task_execution",
        ContextTargetType::ReviewReport => "review_report",
    }
}

fn target_type_from_db(value: &str) -> AppResult<ContextTargetType> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| {
        AppError::Database(format!(
            "Invalid solution critique action target type: {value}"
        ))
    })
}

fn action_to_db(action: SolutionCritiqueGapActionKind) -> &'static str {
    match action {
        SolutionCritiqueGapActionKind::Promoted => "promoted",
        SolutionCritiqueGapActionKind::Deferred => "deferred",
        SolutionCritiqueGapActionKind::Covered => "covered",
        SolutionCritiqueGapActionKind::Reopened => "reopened",
    }
}

fn action_from_db(value: &str) -> AppResult<SolutionCritiqueGapActionKind> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| {
        AppError::Database(format!(
            "Invalid solution critique gap action kind: {value}"
        ))
    })
}

fn parse_datetime(value: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|error| {
            AppError::Database(format!(
                "Invalid solution critique gap action timestamp: {error}"
            ))
        })
}
