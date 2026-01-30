// SQLite-based ProposalDependencyRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;
use uuid::Uuid;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::domain::repositories::ProposalDependencyRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ProposalDependencyRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteProposalDependencyRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProposalDependencyRepository {
    /// Create a new SQLite proposal dependency repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to convert String to TaskProposalId
    fn string_to_proposal_id(s: String) -> TaskProposalId {
        TaskProposalId(s)
    }
}

#[async_trait]
impl ProposalDependencyRepository for SqliteProposalDependencyRepository {
    async fn add_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let id = Uuid::new_v4().to_string();

        // INSERT OR IGNORE to handle UNIQUE constraint gracefully
        conn.execute(
            "INSERT OR IGNORE INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, proposal_id.as_str(), depends_on_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id = ?1 AND depends_on_proposal_id = ?2",
            rusqlite::params![proposal_id.as_str(), depends_on_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_dependencies(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT depends_on_proposal_id FROM proposal_dependencies
                 WHERE proposal_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let deps = stmt
            .query_map([proposal_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_proposal_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(deps)
    }

    async fn get_dependents(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT proposal_id FROM proposal_dependencies
                 WHERE depends_on_proposal_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let dependents = stmt
            .query_map([proposal_id.as_str()], |row| {
                let id: String = row.get(0)?;
                Ok(Self::string_to_proposal_id(id))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(dependents)
    }

    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
        let conn = self.conn.lock().await;

        // Join with task_proposals to filter by session
        let mut stmt = conn
            .prepare(
                "SELECT pd.proposal_id, pd.depends_on_proposal_id
                 FROM proposal_dependencies pd
                 INNER JOIN task_proposals tp ON pd.proposal_id = tp.id
                 WHERE tp.session_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let deps = stmt
            .query_map([session_id.as_str()], |row| {
                let from_id: String = row.get(0)?;
                let to_id: String = row.get(1)?;
                Ok((Self::string_to_proposal_id(from_id), Self::string_to_proposal_id(to_id)))
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(deps)
    }

    async fn would_create_cycle(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<bool> {
        // Self-dependency is always a cycle
        if proposal_id == depends_on_id {
            return Ok(true);
        }

        let conn = self.conn.lock().await;

        // Use DFS to detect if depends_on_id can reach proposal_id
        // If so, adding proposal_id -> depends_on_id would create a cycle
        let mut visited = HashSet::new();
        let mut stack = vec![depends_on_id.clone()];

        while let Some(current) = stack.pop() {
            if current == *proposal_id {
                // We found a path from depends_on_id to proposal_id
                // Adding proposal_id -> depends_on_id would create a cycle
                return Ok(true);
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get all dependencies of current (what current depends on)
            let mut stmt = conn
                .prepare(
                    "SELECT depends_on_proposal_id FROM proposal_dependencies
                     WHERE proposal_id = ?1",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;

            let deps: Vec<TaskProposalId> = stmt
                .query_map([current.as_str()], |row| {
                    let id: String = row.get(0)?;
                    Ok(Self::string_to_proposal_id(id))
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))?;

            for dep in deps {
                if !visited.contains(&dep) {
                    stack.push(dep);
                }
            }
        }

        Ok(false)
    }

    async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Clear both directions: where this proposal depends on others,
        // and where others depend on this proposal
        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id = ?1 OR depends_on_proposal_id = ?1",
            [proposal_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn clear_session_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Delete all dependencies for proposals in this session
        conn.execute(
            "DELETE FROM proposal_dependencies
             WHERE proposal_id IN (
                 SELECT id FROM task_proposals WHERE session_id = ?1
             )",
            [session_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
                [proposal_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE depends_on_proposal_id = ?1",
                [proposal_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        IdeationSession, Priority, ProjectId, TaskCategory, TaskProposal,
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

    fn create_test_session(conn: &Connection, project_id: &ProjectId) -> IdeationSession {
        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Test Session")
            .build();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', datetime('now'), datetime('now'))",
            rusqlite::params![session.id.as_str(), project_id.as_str(), session.title],
        )
        .unwrap();

        session
    }

    fn create_test_proposal(
        conn: &Connection,
        session_id: &IdeationSessionId,
        title: &str,
    ) -> TaskProposal {
        let proposal = TaskProposal::new(
            session_id.clone(),
            title,
            TaskCategory::Feature,
            Priority::Medium,
        );

        conn.execute(
            "INSERT INTO task_proposals (
                id, session_id, title, description, category, suggested_priority,
                priority_score, estimated_complexity, user_modified, status, selected,
                sort_order, created_at, updated_at
            ) VALUES (?1, ?2, ?3, '', 'feature', 'medium', 50, 'moderate', 0, 'pending', 1, 0,
                datetime('now'), datetime('now'))",
            rusqlite::params![proposal.id.as_str(), session_id.as_str(), title],
        )
        .unwrap();

        proposal
    }

    // ==================== ADD DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_add_dependency_creates_record() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let result = repo.add_dependency(&proposal_a.id, &proposal_b.id).await;

        assert!(result.is_ok());

        // Verify dependency was created
        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], proposal_b.id);
    }

    #[tokio::test]
    async fn test_add_dependency_duplicate_is_ignored() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // Add same dependency twice
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        let result = repo.add_dependency(&proposal_a.id, &proposal_b.id).await;

        assert!(result.is_ok());

        // Should only have one dependency
        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(deps.len(), 1);
    }

    #[tokio::test]
    async fn test_add_multiple_dependencies() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A depends on B and C
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_a.id, &proposal_c.id)
            .await
            .unwrap();

        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&proposal_b.id));
        assert!(deps.contains(&proposal_c.id));
    }

    // ==================== REMOVE DEPENDENCY TESTS ====================

    #[tokio::test]
    async fn test_remove_dependency_deletes_record() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        let result = repo.remove_dependency(&proposal_a.id, &proposal_b.id).await;

        assert!(result.is_ok());

        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_dependency_succeeds() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // Should not error
        let result = repo.remove_dependency(&proposal_a.id, &proposal_b.id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_only_specified_dependency() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_a.id, &proposal_c.id)
            .await
            .unwrap();

        // Remove only B dependency
        repo.remove_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();

        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&proposal_c.id));
    }

    // ==================== GET DEPENDENCIES TESTS ====================

    #[tokio::test]
    async fn test_get_dependencies_empty() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let deps = repo.get_dependencies(&proposal.id).await.unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn test_get_dependencies_returns_correct_direction() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A depends on B
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();

        // A's dependencies should include B
        let a_deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(a_deps.len(), 1);
        assert!(a_deps.contains(&proposal_b.id));

        // B should have no dependencies
        let b_deps = repo.get_dependencies(&proposal_b.id).await.unwrap();
        assert!(b_deps.is_empty());
    }

    // ==================== GET DEPENDENTS TESTS ====================

    #[tokio::test]
    async fn test_get_dependents_empty() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let dependents = repo.get_dependents(&proposal.id).await.unwrap();
        assert!(dependents.is_empty());
    }

    #[tokio::test]
    async fn test_get_dependents_returns_correct_direction() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A depends on B (B blocks A)
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();

        // B's dependents should include A
        let b_dependents = repo.get_dependents(&proposal_b.id).await.unwrap();
        assert_eq!(b_dependents.len(), 1);
        assert!(b_dependents.contains(&proposal_a.id));

        // A should have no dependents
        let a_dependents = repo.get_dependents(&proposal_a.id).await.unwrap();
        assert!(a_dependents.is_empty());
    }

    #[tokio::test]
    async fn test_get_dependents_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A and B both depend on C
        repo.add_dependency(&proposal_a.id, &proposal_c.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_b.id, &proposal_c.id)
            .await
            .unwrap();

        let dependents = repo.get_dependents(&proposal_c.id).await.unwrap();
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&proposal_a.id));
        assert!(dependents.contains(&proposal_b.id));
    }

    // ==================== GET ALL FOR SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_all_for_session_empty() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);

        let repo = SqliteProposalDependencyRepository::new(conn);

        let all = repo.get_all_for_session(&session.id).await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_for_session_returns_all_deps() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A -> B, B -> C
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_b.id, &proposal_c.id)
            .await
            .unwrap();

        let all = repo.get_all_for_session(&session.id).await.unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&(proposal_a.id.clone(), proposal_b.id.clone())));
        assert!(all.contains(&(proposal_b.id.clone(), proposal_c.id.clone())));
    }

    #[tokio::test]
    async fn test_get_all_for_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");

        let session1 = create_test_session(&conn, &project_id);
        let session2_id = IdeationSessionId::new();

        // Create another session manually
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, 'Session 2', 'active', datetime('now'), datetime('now'))",
            rusqlite::params![session2_id.as_str(), project_id.as_str()],
        )
        .unwrap();

        let s1_proposal_a = create_test_proposal(&conn, &session1.id, "S1 Proposal A");
        let s1_proposal_b = create_test_proposal(&conn, &session1.id, "S1 Proposal B");
        let s2_proposal_a = create_test_proposal(&conn, &session2_id, "S2 Proposal A");
        let s2_proposal_b = create_test_proposal(&conn, &session2_id, "S2 Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // Create deps in both sessions
        repo.add_dependency(&s1_proposal_a.id, &s1_proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&s2_proposal_a.id, &s2_proposal_b.id)
            .await
            .unwrap();

        // Should only get session 1 deps
        let s1_all = repo.get_all_for_session(&session1.id).await.unwrap();
        assert_eq!(s1_all.len(), 1);
        assert!(s1_all.contains(&(s1_proposal_a.id.clone(), s1_proposal_b.id.clone())));

        // Should only get session 2 deps
        let s2_all = repo.get_all_for_session(&session2_id).await.unwrap();
        assert_eq!(s2_all.len(), 1);
        assert!(s2_all.contains(&(s2_proposal_a.id.clone(), s2_proposal_b.id.clone())));
    }

    // ==================== WOULD CREATE CYCLE TESTS ====================

    #[tokio::test]
    async fn test_would_create_cycle_self_dependency() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let result = repo.would_create_cycle(&proposal.id, &proposal.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_would_create_cycle_direct_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // B depends on A
        repo.add_dependency(&proposal_b.id, &proposal_a.id)
            .await
            .unwrap();

        // Would adding A -> B create a cycle? Yes (A -> B -> A)
        let result = repo.would_create_cycle(&proposal_a.id, &proposal_b.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_would_create_cycle_indirect_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // B -> C, C -> A (existing chain)
        repo.add_dependency(&proposal_b.id, &proposal_c.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_c.id, &proposal_a.id)
            .await
            .unwrap();

        // Would adding A -> B create a cycle? Yes (A -> B -> C -> A)
        let result = repo.would_create_cycle(&proposal_a.id, &proposal_b.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_would_create_cycle_no_cycle() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A -> B (existing)
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();

        // Would adding B -> C create a cycle? No
        let result = repo.would_create_cycle(&proposal_b.id, &proposal_c.id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_would_create_cycle_empty_graph() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // No existing dependencies, would A -> B create a cycle? No
        let result = repo.would_create_cycle(&proposal_a.id, &proposal_b.id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==================== CLEAR DEPENDENCIES TESTS ====================

    #[tokio::test]
    async fn test_clear_dependencies_removes_outgoing() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A -> B, A -> C
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_a.id, &proposal_c.id)
            .await
            .unwrap();

        repo.clear_dependencies(&proposal_a.id).await.unwrap();

        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies_removes_incoming() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // B -> A, C -> A
        repo.add_dependency(&proposal_b.id, &proposal_a.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_c.id, &proposal_a.id)
            .await
            .unwrap();

        repo.clear_dependencies(&proposal_a.id).await.unwrap();

        // A should have no dependents anymore
        let dependents = repo.get_dependents(&proposal_a.id).await.unwrap();
        assert!(dependents.is_empty());

        // B and C should have no dependencies anymore
        let b_deps = repo.get_dependencies(&proposal_b.id).await.unwrap();
        assert!(b_deps.is_empty());
        let c_deps = repo.get_dependencies(&proposal_c.id).await.unwrap();
        assert!(c_deps.is_empty());
    }

    #[tokio::test]
    async fn test_clear_dependencies_removes_both_directions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A -> B (A depends on B), C -> A (C depends on A)
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_c.id, &proposal_a.id)
            .await
            .unwrap();

        repo.clear_dependencies(&proposal_a.id).await.unwrap();

        // A should have no dependencies
        let a_deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert!(a_deps.is_empty());

        // A should have no dependents
        let a_dependents = repo.get_dependents(&proposal_a.id).await.unwrap();
        assert!(a_dependents.is_empty());

        // C should have no dependencies (was depending on A)
        let c_deps = repo.get_dependencies(&proposal_c.id).await.unwrap();
        assert!(c_deps.is_empty());
    }

    // ==================== COUNT TESTS ====================

    #[tokio::test]
    async fn test_count_dependencies_zero() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let count = repo.count_dependencies(&proposal.id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_dependencies_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // A depends on B and C
        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_a.id, &proposal_c.id)
            .await
            .unwrap();

        let count = repo.count_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_dependents_zero() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        let repo = SqliteProposalDependencyRepository::new(conn);

        let count = repo.count_dependents(&proposal.id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_dependents_multiple() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");
        let proposal_c = create_test_proposal(&conn, &session.id, "Proposal C");

        let repo = SqliteProposalDependencyRepository::new(conn);

        // B and C depend on A
        repo.add_dependency(&proposal_b.id, &proposal_a.id)
            .await
            .unwrap();
        repo.add_dependency(&proposal_c.id, &proposal_a.id)
            .await
            .unwrap();

        let count = repo.count_dependents(&proposal_a.id).await.unwrap();
        assert_eq!(count, 2);
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteProposalDependencyRepository::from_shared(shared_conn);

        repo.add_dependency(&proposal_a.id, &proposal_b.id)
            .await
            .unwrap();

        let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
        assert_eq!(deps.len(), 1);
    }

    // ==================== CASCADE DELETE TESTS ====================

    #[tokio::test]
    async fn test_cascade_deletes_when_proposal_deleted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        // Add dependency
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![proposal_a.id.as_str(), proposal_b.id.as_str()],
        )
        .unwrap();

        // Verify dependency exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
                [proposal_a.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Delete proposal A
        conn.execute(
            "DELETE FROM task_proposals WHERE id = ?1",
            [proposal_a.id.as_str()],
        )
        .unwrap();

        // Dependency should be gone due to CASCADE
        let count_after: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
                [proposal_a.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_cascade_deletes_when_depends_on_proposal_deleted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal_a = create_test_proposal(&conn, &session.id, "Proposal A");
        let proposal_b = create_test_proposal(&conn, &session.id, "Proposal B");

        // A depends on B
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![proposal_a.id.as_str(), proposal_b.id.as_str()],
        )
        .unwrap();

        // Delete proposal B
        conn.execute(
            "DELETE FROM task_proposals WHERE id = ?1",
            [proposal_b.id.as_str()],
        )
        .unwrap();

        // Dependency should be gone due to CASCADE
        let count_after: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM proposal_dependencies WHERE depends_on_proposal_id = ?1",
                [proposal_b.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    // ==================== CHECK CONSTRAINT TESTS ====================

    #[tokio::test]
    async fn test_self_dependency_check_constraint() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test", "/test");
        let session = create_test_session(&conn, &project_id);
        let proposal = create_test_proposal(&conn, &session.id, "Proposal");

        // Direct insert should fail due to CHECK constraint
        let result = conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?1)",
            [proposal.id.as_str()],
        );

        assert!(result.is_err());
    }
}
