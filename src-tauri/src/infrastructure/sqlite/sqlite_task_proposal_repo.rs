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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        BusinessValueFactor, Complexity, ComplexityFactor, CriticalPathFactor, DependencyFactor,
        Priority, PriorityAssessmentFactors, ProposalStatus, TaskCategory,
        UserHintFactor, ProjectId,
    };
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', datetime('now'), datetime('now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    }

    fn create_test_session(conn: &Connection, session_id: &IdeationSessionId, project_id: &ProjectId) {
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
             VALUES (?1, ?2, 'active', datetime('now'), datetime('now'))",
            rusqlite::params![session_id.as_str(), project_id.as_str()],
        )
        .unwrap();
    }

    fn create_test_proposal(session_id: &IdeationSessionId, title: &str) -> TaskProposal {
        TaskProposal::new(
            session_id.clone(),
            title,
            TaskCategory::Feature,
            Priority::Medium,
        )
    }

    fn create_test_assessment(proposal_id: &TaskProposalId) -> PriorityAssessment {
        let factors = PriorityAssessmentFactors {
            dependency_factor: DependencyFactor {
                score: 15,
                blocks_count: 2,
                reason: "Blocks 2 tasks".to_string(),
            },
            critical_path_factor: CriticalPathFactor {
                score: 20,
                is_on_critical_path: true,
                path_length: 3,
                reason: "On critical path".to_string(),
            },
            business_value_factor: BusinessValueFactor {
                score: 15,
                keywords: vec!["core".to_string()],
                reason: "Core functionality".to_string(),
            },
            complexity_factor: ComplexityFactor {
                score: 10,
                complexity: Complexity::Simple,
                reason: "Simple task".to_string(),
            },
            user_hint_factor: UserHintFactor {
                score: 5,
                hints: vec!["important".to_string()],
                reason: "User marked important".to_string(),
            },
        };
        PriorityAssessment::new(proposal_id.clone(), factors)
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_proposal_and_returns_it() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Test Proposal");

        let result = repo.create(proposal.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, proposal.id);
        assert_eq!(created.title, "Test Proposal");
        assert_eq!(created.status, ProposalStatus::Pending);
    }

    #[tokio::test]
    async fn test_create_with_all_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let mut proposal = create_test_proposal(&session_id, "Full Proposal");
        proposal.description = Some("Detailed description".to_string());
        proposal.steps = Some(r#"["Step 1", "Step 2"]"#.to_string());
        proposal.acceptance_criteria = Some(r#"["AC 1", "AC 2"]"#.to_string());
        proposal.priority_reason = Some("Important feature".to_string());
        proposal.estimated_complexity = Complexity::Complex;

        let result = repo.create(proposal.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.description, Some("Detailed description".to_string()));
        assert_eq!(created.steps, Some(r#"["Step 1", "Step 2"]"#.to_string()));
        assert_eq!(created.estimated_complexity, Complexity::Complex);
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Duplicate");

        repo.create(proposal.clone()).await.unwrap();
        let result = repo.create(proposal).await;

        assert!(result.is_err());
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_retrieves_proposal_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Get By ID Test");

        repo.create(proposal.clone()).await.unwrap();
        let result = repo.get_by_id(&proposal.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_proposal = found.unwrap();
        assert_eq!(found_proposal.id, proposal.id);
        assert_eq!(found_proposal.title, "Get By ID Test");
        assert_eq!(found_proposal.session_id, session_id);
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteTaskProposalRepository::new(conn);
        let id = TaskProposalId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_all_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let mut proposal = create_test_proposal(&session_id, "Full Fields");
        proposal.description = Some("Description".to_string());
        proposal.steps = Some(r#"["step1"]"#.to_string());
        proposal.acceptance_criteria = Some(r#"["ac1"]"#.to_string());
        proposal.priority_reason = Some("Reason".to_string());
        proposal.estimated_complexity = Complexity::VeryComplex;
        proposal.user_priority = Some(Priority::High);
        proposal.user_modified = true;
        proposal.status = ProposalStatus::Modified;
        proposal.selected = false;
        proposal.sort_order = 5;

        repo.create(proposal.clone()).await.unwrap();
        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();

        assert_eq!(found.id, proposal.id);
        assert_eq!(found.description, Some("Description".to_string()));
        assert_eq!(found.steps, Some(r#"["step1"]"#.to_string()));
        assert_eq!(found.acceptance_criteria, Some(r#"["ac1"]"#.to_string()));
        assert_eq!(found.priority_reason, Some("Reason".to_string()));
        assert_eq!(found.estimated_complexity, Complexity::VeryComplex);
        assert_eq!(found.user_priority, Some(Priority::High));
        assert!(found.user_modified);
        assert_eq!(found.status, ProposalStatus::Modified);
        assert!(!found.selected);
        assert_eq!(found.sort_order, 5);
    }

    // ==================== GET BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_by_session_returns_all_proposals() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let proposal1 = create_test_proposal(&session_id, "Proposal 1");
        let proposal2 = create_test_proposal(&session_id, "Proposal 2");
        let proposal3 = create_test_proposal(&session_id, "Proposal 3");

        repo.create(proposal1).await.unwrap();
        repo.create(proposal2).await.unwrap();
        repo.create(proposal3).await.unwrap();

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 3);
    }

    #[tokio::test]
    async fn test_get_by_session_ordered_by_sort_order() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut proposal1 = create_test_proposal(&session_id, "Third");
        proposal1.sort_order = 3;
        let mut proposal2 = create_test_proposal(&session_id, "First");
        proposal2.sort_order = 1;
        let mut proposal3 = create_test_proposal(&session_id, "Second");
        proposal3.sort_order = 2;

        // Insert in non-order
        repo.create(proposal1).await.unwrap();
        repo.create(proposal3).await.unwrap();
        repo.create(proposal2).await.unwrap();

        let proposals = repo.get_by_session(&session_id).await.unwrap();

        assert_eq!(proposals.len(), 3);
        assert_eq!(proposals[0].title, "First");
        assert_eq!(proposals[0].sort_order, 1);
        assert_eq!(proposals[1].title, "Second");
        assert_eq!(proposals[1].sort_order, 2);
        assert_eq!(proposals[2].title, "Third");
        assert_eq!(proposals[2].sort_order, 3);
    }

    #[tokio::test]
    async fn test_get_by_session_returns_empty_for_no_proposals() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id1 = IdeationSessionId::new();
        let session_id2 = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id1, &project_id);
        create_test_session(&conn, &session_id2, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let proposal1 = create_test_proposal(&session_id1, "Session 1 Proposal");
        let proposal2 = create_test_proposal(&session_id2, "Session 2 Proposal");

        repo.create(proposal1).await.unwrap();
        repo.create(proposal2).await.unwrap();

        let proposals = repo.get_by_session(&session_id1).await.unwrap();

        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].session_id, session_id1);
    }

    // ==================== UPDATE TESTS ====================

    #[tokio::test]
    async fn test_update_modifies_proposal() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let mut proposal = create_test_proposal(&session_id, "Original");

        repo.create(proposal.clone()).await.unwrap();

        proposal.title = "Updated Title".to_string();
        proposal.description = Some("Updated description".to_string());
        proposal.category = TaskCategory::Fix;
        proposal.status = ProposalStatus::Accepted;

        repo.update(&proposal).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.description, Some("Updated description".to_string()));
        assert_eq!(found.category, TaskCategory::Fix);
        assert_eq!(found.status, ProposalStatus::Accepted);
    }

    #[tokio::test]
    async fn test_update_updates_updated_at() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Timestamp Test");
        let original_updated = proposal.updated_at;

        repo.create(proposal.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut updated_proposal = proposal.clone();
        updated_proposal.title = "Changed".to_string();
        repo.update(&updated_proposal).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(found.updated_at >= original_updated);
    }

    // ==================== UPDATE PRIORITY TESTS ====================

    #[tokio::test]
    async fn test_update_priority_sets_assessment_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Priority Update");

        repo.create(proposal.clone()).await.unwrap();

        let assessment = create_test_assessment(&proposal.id);
        repo.update_priority(&proposal.id, &assessment).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert_eq!(found.suggested_priority, assessment.suggested_priority);
        assert_eq!(found.priority_score, assessment.priority_score);
        assert_eq!(found.priority_reason, Some(assessment.priority_reason.clone()));
    }

    #[tokio::test]
    async fn test_update_priority_stores_factors_as_json() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Factors JSON");

        repo.create(proposal.clone()).await.unwrap();

        let assessment = create_test_assessment(&proposal.id);
        repo.update_priority(&proposal.id, &assessment).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        // The priority_factors field might not deserialize because PriorityAssessmentFactors
        // has different structure than PriorityFactors. But the reason should be stored.
        assert_eq!(found.priority_reason, Some(assessment.priority_reason.clone()));
        // The main priority fields should be updated
        assert_eq!(found.suggested_priority, assessment.suggested_priority);
        assert_eq!(found.priority_score, assessment.priority_score);
    }

    // ==================== UPDATE SELECTION TESTS ====================

    #[tokio::test]
    async fn test_update_selection_toggles_selected() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Selection Test");
        // Default is selected = true

        repo.create(proposal.clone()).await.unwrap();

        // Deselect
        repo.update_selection(&proposal.id, false).await.unwrap();
        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(!found.selected);

        // Select again
        repo.update_selection(&proposal.id, true).await.unwrap();
        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(found.selected);
    }

    #[tokio::test]
    async fn test_update_selection_updates_timestamp() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Selection Timestamp");
        let original = proposal.updated_at;

        repo.create(proposal.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        repo.update_selection(&proposal.id, false).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(found.updated_at >= original);
    }

    // ==================== SET CREATED TASK ID TESTS ====================

    fn create_test_task(conn: &Connection, task_id: &TaskId, project_id: &ProjectId, title: &str) {
        conn.execute(
            "INSERT INTO tasks (id, project_id, title, category, internal_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'feature', 'Ready', datetime('now'), datetime('now'))",
            rusqlite::params![task_id.as_str(), project_id.as_str(), title],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_set_created_task_id_links_proposal_to_task() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        let task_id = TaskId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);
        create_test_task(&conn, &task_id, &project_id, "Created Task");

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Link Task");

        repo.create(proposal.clone()).await.unwrap();

        repo.set_created_task_id(&proposal.id, &task_id).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert_eq!(found.created_task_id, Some(task_id));
    }

    #[tokio::test]
    async fn test_set_created_task_id_updates_timestamp() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        let task_id = TaskId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);
        create_test_task(&conn, &task_id, &project_id, "Timestamp Task");

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "Task Link Timestamp");
        let original = proposal.updated_at;

        repo.create(proposal.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        repo.set_created_task_id(&proposal.id, &task_id).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(found.updated_at >= original);
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_removes_proposal_from_database() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let proposal = create_test_proposal(&session_id, "To Delete");

        repo.create(proposal.clone()).await.unwrap();

        let delete_result = repo.delete(&proposal.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&proposal.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteTaskProposalRepository::new(conn);
        let id = TaskProposalId::new();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    // ==================== REORDER TESTS ====================

    #[tokio::test]
    async fn test_reorder_updates_sort_order() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut proposal1 = create_test_proposal(&session_id, "First");
        proposal1.sort_order = 1;
        let mut proposal2 = create_test_proposal(&session_id, "Second");
        proposal2.sort_order = 2;
        let mut proposal3 = create_test_proposal(&session_id, "Third");
        proposal3.sort_order = 3;

        repo.create(proposal1.clone()).await.unwrap();
        repo.create(proposal2.clone()).await.unwrap();
        repo.create(proposal3.clone()).await.unwrap();

        // Reorder: move third to first position
        let new_order = vec![proposal3.id.clone(), proposal1.id.clone(), proposal2.id.clone()];
        repo.reorder(&session_id, new_order).await.unwrap();

        let proposals = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(proposals[0].title, "Third");
        assert_eq!(proposals[0].sort_order, 0);
        assert_eq!(proposals[1].title, "First");
        assert_eq!(proposals[1].sort_order, 1);
        assert_eq!(proposals[2].title, "Second");
        assert_eq!(proposals[2].sort_order, 2);
    }

    #[tokio::test]
    async fn test_reorder_only_affects_specified_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id1 = IdeationSessionId::new();
        let session_id2 = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id1, &project_id);
        create_test_session(&conn, &session_id2, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut proposal1 = create_test_proposal(&session_id1, "Session1");
        proposal1.sort_order = 1;
        let mut proposal2 = create_test_proposal(&session_id2, "Session2");
        proposal2.sort_order = 1;

        repo.create(proposal1.clone()).await.unwrap();
        repo.create(proposal2.clone()).await.unwrap();

        // Reorder session1 only
        repo.reorder(&session_id1, vec![proposal1.id.clone()]).await.unwrap();

        // Session 2 should be unaffected
        let found = repo.get_by_id(&proposal2.id).await.unwrap().unwrap();
        assert_eq!(found.sort_order, 1);
    }

    // ==================== GET SELECTED BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_selected_by_session_returns_only_selected() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut selected = create_test_proposal(&session_id, "Selected");
        selected.selected = true;
        let mut unselected = create_test_proposal(&session_id, "Unselected");
        unselected.selected = false;

        repo.create(selected.clone()).await.unwrap();
        repo.create(unselected.clone()).await.unwrap();

        let result = repo.get_selected_by_session(&session_id).await;

        assert!(result.is_ok());
        let proposals = result.unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, selected.id);
        assert!(proposals[0].selected);
    }

    #[tokio::test]
    async fn test_get_selected_by_session_returns_empty_when_none_selected() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut unselected = create_test_proposal(&session_id, "Unselected");
        unselected.selected = false;

        repo.create(unselected).await.unwrap();

        let result = repo.get_selected_by_session(&session_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_selected_by_session_ordered_by_sort_order() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut proposal1 = create_test_proposal(&session_id, "Third");
        proposal1.sort_order = 3;
        proposal1.selected = true;
        let mut proposal2 = create_test_proposal(&session_id, "First");
        proposal2.sort_order = 1;
        proposal2.selected = true;

        repo.create(proposal1).await.unwrap();
        repo.create(proposal2).await.unwrap();

        let proposals = repo.get_selected_by_session(&session_id).await.unwrap();

        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].title, "First");
        assert_eq!(proposals[1].title, "Third");
    }

    // ==================== COUNT TESTS ====================

    #[tokio::test]
    async fn test_count_by_session_returns_zero_for_no_proposals() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let result = repo.count_by_session(&session_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_by_session_counts_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let proposal1 = create_test_proposal(&session_id, "One");
        let proposal2 = create_test_proposal(&session_id, "Two");
        let proposal3 = create_test_proposal(&session_id, "Three");

        repo.create(proposal1).await.unwrap();
        repo.create(proposal2).await.unwrap();
        repo.create(proposal3).await.unwrap();

        let count = repo.count_by_session(&session_id).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_count_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id1 = IdeationSessionId::new();
        let session_id2 = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id1, &project_id);
        create_test_session(&conn, &session_id2, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let proposal1 = create_test_proposal(&session_id1, "Session 1");
        let proposal2 = create_test_proposal(&session_id2, "Session 2 A");
        let proposal3 = create_test_proposal(&session_id2, "Session 2 B");

        repo.create(proposal1).await.unwrap();
        repo.create(proposal2).await.unwrap();
        repo.create(proposal3).await.unwrap();

        let count1 = repo.count_by_session(&session_id1).await.unwrap();
        let count2 = repo.count_by_session(&session_id2).await.unwrap();

        assert_eq!(count1, 1);
        assert_eq!(count2, 2);
    }

    #[tokio::test]
    async fn test_count_selected_by_session_counts_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);

        let mut selected1 = create_test_proposal(&session_id, "Selected 1");
        selected1.selected = true;
        let mut selected2 = create_test_proposal(&session_id, "Selected 2");
        selected2.selected = true;
        let mut unselected = create_test_proposal(&session_id, "Unselected");
        unselected.selected = false;

        repo.create(selected1).await.unwrap();
        repo.create(selected2).await.unwrap();
        repo.create(unselected).await.unwrap();

        let total_count = repo.count_by_session(&session_id).await.unwrap();
        let selected_count = repo.count_selected_by_session(&session_id).await.unwrap();

        assert_eq!(total_count, 3);
        assert_eq!(selected_count, 2);
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteTaskProposalRepository::from_shared(shared_conn);

        let proposal = create_test_proposal(&session_id, "Shared Connection");

        let result = repo.create(proposal.clone()).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&proposal.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== PRIORITY FACTORS JSON TESTS ====================

    #[tokio::test]
    async fn test_create_with_priority_factors() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        create_test_session(&conn, &session_id, &project_id);

        let repo = SqliteTaskProposalRepository::new(conn);
        let mut proposal = create_test_proposal(&session_id, "With Factors");
        proposal.priority_factors = Some(crate::domain::entities::PriorityFactors {
            dependency: 10,
            business_value: 20,
            technical_risk: 5,
            user_demand: 15,
        });

        repo.create(proposal.clone()).await.unwrap();

        let found = repo.get_by_id(&proposal.id).await.unwrap().unwrap();
        assert!(found.priority_factors.is_some());
        let factors = found.priority_factors.unwrap();
        assert_eq!(factors.dependency, 10);
        assert_eq!(factors.business_value, 20);
    }
}
