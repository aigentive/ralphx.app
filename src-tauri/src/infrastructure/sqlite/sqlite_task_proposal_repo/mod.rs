// SQLite-based TaskProposalRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use crate::domain::entities::{
    ArtifactId, IdeationSessionId, PriorityAssessment, TaskId, TaskProposal, TaskProposalId,
};
use crate::domain::repositories::TaskProposalRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of TaskProposalRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteTaskProposalRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskProposalRepository {
    /// Create a new SQLite task proposal repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TaskProposalRepository for SqliteTaskProposalRepository {
    async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
        let conn = self.conn.lock().await;

        // Serialize priority_factors to JSON if present
        let priority_factors_json = proposal
            .priority_factors
            .as_ref()
            .and_then(|f| serde_json::to_string(f).ok());

        conn.execute(
            "INSERT INTO task_proposals (
                id, session_id, title, description, category, steps, acceptance_criteria,
                suggested_priority, priority_score, priority_reason, priority_factors,
                estimated_complexity, user_priority, user_modified, status, selected,
                created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
            )",
            rusqlite::params![
                proposal.id.as_str(),
                proposal.session_id.as_str(),
                proposal.title,
                proposal.description,
                proposal.category.to_string(),
                proposal.steps,
                proposal.acceptance_criteria,
                proposal.suggested_priority.to_string(),
                proposal.priority_score,
                proposal.priority_reason,
                priority_factors_json,
                proposal.estimated_complexity.to_string(),
                proposal.user_priority.map(|p| p.to_string()),
                proposal.user_modified as i32,
                proposal.status.to_string(),
                proposal.selected as i32,
                proposal.created_task_id.as_ref().map(|id| id.as_str()),
                proposal.plan_artifact_id.as_ref().map(|id| id.as_str()),
                proposal.plan_version_at_creation,
                proposal.sort_order,
                proposal.created_at.to_rfc3339(),
                proposal.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(proposal)
    }

    async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, session_id, title, description, category, steps, acceptance_criteria,
                    suggested_priority, priority_score, priority_reason, priority_factors,
                    estimated_complexity, user_priority, user_modified, status, selected,
                    created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
             FROM task_proposals WHERE id = ?1",
            [id.as_str()],
            |row| TaskProposal::from_row(row),
        );

        match result {
            Ok(proposal) => Ok(Some(proposal)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, title, description, category, steps, acceptance_criteria,
                        suggested_priority, priority_score, priority_reason, priority_factors,
                        estimated_complexity, user_priority, user_modified, status, selected,
                        created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
                 FROM task_proposals WHERE session_id = ?1 ORDER BY sort_order ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let proposals = stmt
            .query_map([session_id.as_str()], TaskProposal::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(proposals)
    }

    async fn update(&self, proposal: &TaskProposal) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        // Serialize priority_factors to JSON if present
        let priority_factors_json = proposal
            .priority_factors
            .as_ref()
            .and_then(|f| serde_json::to_string(f).ok());

        conn.execute(
            "UPDATE task_proposals SET
                title = ?2, description = ?3, category = ?4, steps = ?5, acceptance_criteria = ?6,
                suggested_priority = ?7, priority_score = ?8, priority_reason = ?9, priority_factors = ?10,
                estimated_complexity = ?11, user_priority = ?12, user_modified = ?13, status = ?14,
                selected = ?15, created_task_id = ?16, plan_artifact_id = ?17, plan_version_at_creation = ?18,
                sort_order = ?19, updated_at = ?20
             WHERE id = ?1",
            rusqlite::params![
                proposal.id.as_str(),
                proposal.title,
                proposal.description,
                proposal.category.to_string(),
                proposal.steps,
                proposal.acceptance_criteria,
                proposal.suggested_priority.to_string(),
                proposal.priority_score,
                proposal.priority_reason,
                priority_factors_json,
                proposal.estimated_complexity.to_string(),
                proposal.user_priority.map(|p| p.to_string()),
                proposal.user_modified as i32,
                proposal.status.to_string(),
                proposal.selected as i32,
                proposal.created_task_id.as_ref().map(|id| id.as_str()),
                proposal.plan_artifact_id.as_ref().map(|id| id.as_str()),
                proposal.plan_version_at_creation,
                proposal.sort_order,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_priority(
        &self,
        id: &TaskProposalId,
        assessment: &PriorityAssessment,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        // Serialize factors to JSON
        let factors_json = serde_json::to_string(&assessment.factors)
            .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute(
            "UPDATE task_proposals SET
                suggested_priority = ?2, priority_score = ?3, priority_reason = ?4,
                priority_factors = ?5, updated_at = ?6
             WHERE id = ?1",
            rusqlite::params![
                id.as_str(),
                assessment.suggested_priority.to_string(),
                assessment.priority_score,
                assessment.priority_reason,
                factors_json,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE task_proposals SET selected = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), selected as i32, now.to_rfc3339(),],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn set_created_task_id(
        &self,
        id: &TaskProposalId,
        task_id: &TaskId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE task_proposals SET created_task_id = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), task_id.as_str(), now.to_rfc3339(),],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &TaskProposalId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // CASCADE is defined in the schema, so deleting the proposal
        // will automatically delete related dependencies
        conn.execute("DELETE FROM task_proposals WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn reorder(
        &self,
        session_id: &IdeationSessionId,
        proposal_ids: Vec<TaskProposalId>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        // Update sort_order for each proposal based on position in the provided list
        for (index, proposal_id) in proposal_ids.iter().enumerate() {
            conn.execute(
                "UPDATE task_proposals SET sort_order = ?2, updated_at = ?3
                 WHERE id = ?1 AND session_id = ?4",
                rusqlite::params![
                    proposal_id.as_str(),
                    index as i32,
                    now.to_rfc3339(),
                    session_id.as_str(),
                ],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_selected_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<TaskProposal>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, title, description, category, steps, acceptance_criteria,
                        suggested_priority, priority_score, priority_reason, priority_factors,
                        estimated_complexity, user_priority, user_modified, status, selected,
                        created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
                 FROM task_proposals
                 WHERE session_id = ?1 AND selected = 1
                 ORDER BY sort_order ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let proposals = stmt
            .query_map([session_id.as_str()], TaskProposal::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(proposals)
    }

    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_proposals WHERE session_id = ?1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_proposals WHERE session_id = ?1 AND selected = 1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn get_by_plan_artifact_id(&self, artifact_id: &ArtifactId) -> AppResult<Vec<TaskProposal>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, title, description, category, steps, acceptance_criteria,
                        suggested_priority, priority_score, priority_reason, priority_factors,
                        estimated_complexity, user_priority, user_modified, status, selected,
                        created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
                 FROM task_proposals
                 WHERE plan_artifact_id = ?1
                 ORDER BY sort_order ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let proposals = stmt
            .query_map([artifact_id.as_str()], TaskProposal::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(proposals)
    }

    async fn clear_created_task_ids_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        conn.execute(
            "UPDATE task_proposals SET created_task_id = NULL, updated_at = ?2 WHERE session_id = ?1",
            rusqlite::params![session_id.as_str(), now.to_rfc3339()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

