// Tests for SqliteProposalDependencyRepository

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, Priority, ProjectId, ProposalCategory, TaskProposal,
};
use crate::domain::repositories::ProposalDependencyRepository;
use crate::infrastructure::sqlite::SqliteProposalDependencyRepository;
use crate::testing::SqliteTestDb;

fn setup_test_db() -> SqliteTestDb {
    SqliteTestDb::new("sqlite-proposal-dependency-repo")
}

fn create_test_project(db: &SqliteTestDb, id: &ProjectId, name: &str, path: &str) {
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    });
}

fn create_test_session(db: &SqliteTestDb, project_id: &ProjectId) -> IdeationSession {
    let session = IdeationSession::builder()
        .project_id(project_id.clone())
        .title("Test Session")
        .build();

    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![session.id.as_str(), project_id.as_str(), session.title],
        )
        .unwrap();
    });

    session
}

fn create_test_proposal(
    db: &SqliteTestDb,
    session_id: &IdeationSessionId,
    title: &str,
) -> TaskProposal {
    let proposal = TaskProposal::new(
        session_id.clone(),
        title,
        ProposalCategory::Feature,
        Priority::Medium,
    );

    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO task_proposals (
                id, session_id, title, description, category, suggested_priority,
                priority_score, estimated_complexity, user_modified, status, selected,
                sort_order, created_at, updated_at
            ) VALUES (?1, ?2, ?3, '', 'feature', 'medium', 50, 'moderate', 0, 'pending', 1, 0,
                strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![proposal.id.as_str(), session_id.as_str(), title],
        )
        .unwrap();
    });

    proposal
}

// ==================== ADD DEPENDENCY TESTS ====================

#[tokio::test]
async fn test_add_dependency_creates_record() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let result = repo
        .add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await;

    assert!(result.is_ok());

    // Verify dependency was created
    let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], proposal_b.id);
}

#[tokio::test]
async fn test_add_dependency_duplicate_is_ignored() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Add same dependency twice
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    let result = repo
        .add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await;

    assert!(result.is_ok());

    // Should only have one dependency
    let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
    assert_eq!(deps.len(), 1);
}

#[tokio::test]
async fn test_add_multiple_dependencies() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A depends on B and C
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_a.id, &proposal_c.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    let result = repo.remove_dependency(&proposal_a.id, &proposal_b.id).await;

    assert!(result.is_ok());

    let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
    assert!(deps.is_empty());
}

#[tokio::test]
async fn test_remove_nonexistent_dependency_succeeds() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Should not error
    let result = repo.remove_dependency(&proposal_a.id, &proposal_b.id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_only_specified_dependency() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_a.id, &proposal_c.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let deps = repo.get_dependencies(&proposal.id).await.unwrap();
    assert!(deps.is_empty());
}

#[tokio::test]
async fn test_get_dependencies_returns_correct_direction() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A depends on B
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let dependents = repo.get_dependents(&proposal.id).await.unwrap();
    assert!(dependents.is_empty());
}

#[tokio::test]
async fn test_get_dependents_returns_correct_direction() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A depends on B (B blocks A)
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A and B both depend on C
    repo.add_dependency(&proposal_a.id, &proposal_c.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_b.id, &proposal_c.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let all = repo.get_all_for_session(&session.id).await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_get_all_for_session_returns_all_deps() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A -> B, B -> C
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_b.id, &proposal_c.id, None, None)
        .await
        .unwrap();

    let all = repo.get_all_for_session(&session.id).await.unwrap();
    assert_eq!(all.len(), 2);
    // Check that the dependencies exist (reason is None since we passed None)
    assert!(all
        .iter()
        .any(|(from, to, _)| from == &proposal_a.id && to == &proposal_b.id));
    assert!(all
        .iter()
        .any(|(from, to, _)| from == &proposal_b.id && to == &proposal_c.id));
}

#[tokio::test]
async fn test_get_all_for_session_filters_by_session() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");

    let session1 = create_test_session(&db, &project_id);
    let session2_id = IdeationSessionId::new();

    // Create another session manually
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, 'Session 2', 'active', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![session2_id.as_str(), project_id.as_str()],
        )
        .unwrap();
    });

    let s1_proposal_a = create_test_proposal(&db, &session1.id, "S1 Proposal A");
    let s1_proposal_b = create_test_proposal(&db, &session1.id, "S1 Proposal B");
    let s2_proposal_a = create_test_proposal(&db, &session2_id, "S2 Proposal A");
    let s2_proposal_b = create_test_proposal(&db, &session2_id, "S2 Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Create deps in both sessions
    repo.add_dependency(&s1_proposal_a.id, &s1_proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&s2_proposal_a.id, &s2_proposal_b.id, None, None)
        .await
        .unwrap();

    // Should only get session 1 deps
    let s1_all = repo.get_all_for_session(&session1.id).await.unwrap();
    assert_eq!(s1_all.len(), 1);
    assert!(s1_all
        .iter()
        .any(|(from, to, _)| from == &s1_proposal_a.id && to == &s1_proposal_b.id));

    // Should only get session 2 deps
    let s2_all = repo.get_all_for_session(&session2_id).await.unwrap();
    assert_eq!(s2_all.len(), 1);
    assert!(s2_all
        .iter()
        .any(|(from, to, _)| from == &s2_proposal_a.id && to == &s2_proposal_b.id));
}

// ==================== WOULD CREATE CYCLE TESTS ====================

#[tokio::test]
async fn test_would_create_cycle_self_dependency() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let result = repo.would_create_cycle(&proposal.id, &proposal.id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_would_create_cycle_direct_cycle() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // B depends on A
    repo.add_dependency(&proposal_b.id, &proposal_a.id, None, None)
        .await
        .unwrap();

    // Would adding A -> B create a cycle? Yes (A -> B -> A)
    let result = repo
        .would_create_cycle(&proposal_a.id, &proposal_b.id)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_would_create_cycle_indirect_cycle() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // B -> C, C -> A (existing chain)
    repo.add_dependency(&proposal_b.id, &proposal_c.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_c.id, &proposal_a.id, None, None)
        .await
        .unwrap();

    // Would adding A -> B create a cycle? Yes (A -> B -> C -> A)
    let result = repo
        .would_create_cycle(&proposal_a.id, &proposal_b.id)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_would_create_cycle_no_cycle() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A -> B (existing)
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();

    // Would adding B -> C create a cycle? No
    let result = repo
        .would_create_cycle(&proposal_b.id, &proposal_c.id)
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_would_create_cycle_empty_graph() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // No existing dependencies, would A -> B create a cycle? No
    let result = repo
        .would_create_cycle(&proposal_a.id, &proposal_b.id)
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ==================== CLEAR DEPENDENCIES TESTS ====================

#[tokio::test]
async fn test_clear_dependencies_removes_outgoing() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A -> B, A -> C
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_a.id, &proposal_c.id, None, None)
        .await
        .unwrap();

    repo.clear_dependencies(&proposal_a.id).await.unwrap();

    let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
    assert!(deps.is_empty());
}

#[tokio::test]
async fn test_clear_dependencies_removes_incoming() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // B -> A, C -> A
    repo.add_dependency(&proposal_b.id, &proposal_a.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_c.id, &proposal_a.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A -> B (A depends on B), C -> A (C depends on A)
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_c.id, &proposal_a.id, None, None)
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
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let count = repo.count_dependencies(&proposal.id).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_count_dependencies_multiple() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // A depends on B and C
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_a.id, &proposal_c.id, None, None)
        .await
        .unwrap();

    let count = repo.count_dependencies(&proposal_a.id).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_count_dependents_zero() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    let count = repo.count_dependents(&proposal.id).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_count_dependents_multiple() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // B and C depend on A
    repo.add_dependency(&proposal_b.id, &proposal_a.id, None, None)
        .await
        .unwrap();
    repo.add_dependency(&proposal_c.id, &proposal_a.id, None, None)
        .await
        .unwrap();

    let count = repo.count_dependents(&proposal_a.id).await.unwrap();
    assert_eq!(count, 2);
}

// ==================== SHARED CONNECTION TESTS ====================

#[tokio::test]
async fn test_from_shared_works_correctly() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::from_shared(db.shared_conn());

    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();

    let deps = repo.get_dependencies(&proposal_a.id).await.unwrap();
    assert_eq!(deps.len(), 1);
}

// ==================== CASCADE DELETE TESTS ====================

#[tokio::test]
async fn test_cascade_deletes_when_proposal_deleted() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    // Add dependency
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![proposal_a.id.as_str(), proposal_b.id.as_str()],
        )
        .unwrap();
    });

    // Verify dependency exists
    let count: i32 = db.with_connection(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
            [proposal_a.id.as_str()],
            |row| row.get(0),
        )
        .unwrap()
    });
    assert_eq!(count, 1);

    // Delete proposal A
    db.with_connection(|conn| {
        conn.execute(
            "DELETE FROM task_proposals WHERE id = ?1",
            [proposal_a.id.as_str()],
        )
        .unwrap();
    });

    // Dependency should be gone due to CASCADE
    let count_after: i32 = db.with_connection(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM proposal_dependencies WHERE proposal_id = ?1",
            [proposal_a.id.as_str()],
            |row| row.get(0),
        )
        .unwrap()
    });
    assert_eq!(count_after, 0);
}

#[tokio::test]
async fn test_cascade_deletes_when_depends_on_proposal_deleted() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    // A depends on B
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?2)",
            rusqlite::params![proposal_a.id.as_str(), proposal_b.id.as_str()],
        )
        .unwrap();
    });

    // Delete proposal B
    db.with_connection(|conn| {
        conn.execute(
            "DELETE FROM task_proposals WHERE id = ?1",
            [proposal_b.id.as_str()],
        )
        .unwrap();
    });

    // Dependency should be gone due to CASCADE
    let count_after: i32 = db.with_connection(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM proposal_dependencies WHERE depends_on_proposal_id = ?1",
            [proposal_b.id.as_str()],
            |row| row.get(0),
        )
        .unwrap()
    });
    assert_eq!(count_after, 0);
}

// ==================== CHECK CONSTRAINT TESTS ====================

#[tokio::test]
async fn test_self_dependency_check_constraint() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal = create_test_proposal(&db, &session.id, "Proposal");

    // Direct insert should fail due to CHECK constraint
    let result = db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', ?1, ?1)",
            [proposal.id.as_str()],
        )
    });

    assert!(result.is_err());
}

// ==================== SOURCE-AWARE METHODS TESTS ====================

#[tokio::test]
async fn test_get_all_for_session_with_source_includes_source_field() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Add auto-suggested dependency
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, Some("auto"))
        .await
        .unwrap();

    let all = repo
        .get_all_for_session_with_source(&session.id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, proposal_a.id);
    assert_eq!(all[0].1, proposal_b.id);
    assert_eq!(all[0].3, "auto");
}

#[tokio::test]
async fn test_add_dependency_with_manual_source() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Add manual dependency
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, Some("manual"))
        .await
        .unwrap();

    let all = repo
        .get_all_for_session_with_source(&session.id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].3, "manual");
}

#[tokio::test]
async fn test_add_dependency_defaults_to_auto() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Add dependency with None source (should default to "auto")
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, None)
        .await
        .unwrap();

    let all = repo
        .get_all_for_session_with_source(&session.id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].3, "auto");
}

#[tokio::test]
async fn test_clear_auto_dependencies_preserves_manual_deps() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Add auto dependency: A -> B
    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, Some("auto"))
        .await
        .unwrap();
    // Add manual dependency: B -> C
    repo.add_dependency(&proposal_b.id, &proposal_c.id, None, Some("manual"))
        .await
        .unwrap();

    // Clear only auto dependencies
    repo.clear_auto_dependencies(&session.id).await.unwrap();

    let all = repo
        .get_all_for_session_with_source(&session.id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    // Only the manual dependency should remain
    assert_eq!(all[0].0, proposal_b.id);
    assert_eq!(all[0].1, proposal_c.id);
    assert_eq!(all[0].3, "manual");
}

#[tokio::test]
async fn test_clear_auto_dependencies_clears_only_in_session() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");

    let session1 = create_test_session(&db, &project_id);
    let session2_id = IdeationSessionId::new();

    // Create another session manually
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, 'Session 2', 'active', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![session2_id.as_str(), project_id.as_str()],
        )
        .unwrap();
    });

    let s1_proposal_a = create_test_proposal(&db, &session1.id, "S1 Proposal A");
    let s1_proposal_b = create_test_proposal(&db, &session1.id, "S1 Proposal B");
    let s2_proposal_a = create_test_proposal(&db, &session2_id, "S2 Proposal A");
    let s2_proposal_b = create_test_proposal(&db, &session2_id, "S2 Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Create auto deps in both sessions
    repo.add_dependency(&s1_proposal_a.id, &s1_proposal_b.id, None, Some("auto"))
        .await
        .unwrap();
    repo.add_dependency(&s2_proposal_a.id, &s2_proposal_b.id, None, Some("auto"))
        .await
        .unwrap();

    // Clear auto deps only for session 1
    repo.clear_auto_dependencies(&session1.id).await.unwrap();

    // Session 1 should have no deps
    let s1_all = repo
        .get_all_for_session_with_source(&session1.id)
        .await
        .unwrap();
    assert_eq!(s1_all.len(), 0);

    // Session 2 should still have its auto dep
    let s2_all = repo
        .get_all_for_session_with_source(&session2_id)
        .await
        .unwrap();
    assert_eq!(s2_all.len(), 1);
    assert_eq!(s2_all[0].3, "auto");
}

#[tokio::test]
async fn test_get_all_for_session_with_source_filters_by_session() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");

    let session1 = create_test_session(&db, &project_id);
    let session2_id = IdeationSessionId::new();

    // Create another session manually
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, 'Session 2', 'active', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
            rusqlite::params![session2_id.as_str(), project_id.as_str()],
        )
        .unwrap();
    });

    let s1_proposal_a = create_test_proposal(&db, &session1.id, "S1 Proposal A");
    let s1_proposal_b = create_test_proposal(&db, &session1.id, "S1 Proposal B");
    let s2_proposal_a = create_test_proposal(&db, &session2_id, "S2 Proposal A");
    let s2_proposal_b = create_test_proposal(&db, &session2_id, "S2 Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // Create deps in both sessions
    repo.add_dependency(&s1_proposal_a.id, &s1_proposal_b.id, None, Some("auto"))
        .await
        .unwrap();
    repo.add_dependency(&s2_proposal_a.id, &s2_proposal_b.id, None, Some("manual"))
        .await
        .unwrap();

    // Should only get session 1 deps
    let s1_all = repo
        .get_all_for_session_with_source(&session1.id)
        .await
        .unwrap();
    assert_eq!(s1_all.len(), 1);
    assert_eq!(s1_all[0].3, "auto");

    // Should only get session 2 deps
    let s2_all = repo
        .get_all_for_session_with_source(&session2_id)
        .await
        .unwrap();
    assert_eq!(s2_all.len(), 1);
    assert_eq!(s2_all[0].3, "manual");
}

#[tokio::test]
async fn test_get_all_for_session_ignores_archived_proposal_endpoints() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");
    let proposal_c = create_test_proposal(&db, &session.id, "Proposal C");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    repo.add_dependency(&proposal_a.id, &proposal_b.id, None, Some("auto"))
        .await
        .unwrap();
    repo.add_dependency(&proposal_c.id, &proposal_a.id, None, Some("manual"))
        .await
        .unwrap();

    repo.db
        .run({
            let proposal_a_id = proposal_a.id.as_str().to_string();
            move |conn| {
                conn.execute(
                    "UPDATE task_proposals
                     SET archived_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                     WHERE id = ?1",
                    rusqlite::params![proposal_a_id],
                )?;
                Ok(())
            }
        })
        .await
        .unwrap();

    let deps = repo.get_all_for_session(&session.id).await.unwrap();
    assert!(
        deps.is_empty(),
        "archived proposals must be ignored whether they are the source or target"
    );

    let deps_with_source = repo
        .get_all_for_session_with_source(&session.id)
        .await
        .unwrap();
    assert!(
        deps_with_source.is_empty(),
        "archived proposals must be ignored in source-aware query"
    );
}

#[tokio::test]
async fn test_would_create_cycle_includes_both_auto_and_manual() {
    let db = setup_test_db();
    let project_id = ProjectId::new();
    create_test_project(&db, &project_id, "Test", "/test");
    let session = create_test_session(&db, &project_id);
    let proposal_a = create_test_proposal(&db, &session.id, "Proposal A");
    let proposal_b = create_test_proposal(&db, &session.id, "Proposal B");

    let repo = SqliteProposalDependencyRepository::new(db.new_connection());

    // B depends on A (manual)
    repo.add_dependency(&proposal_b.id, &proposal_a.id, None, Some("manual"))
        .await
        .unwrap();

    // Would adding A -> B create a cycle? Yes (A -> B -> A)
    // Should detect cycle regardless of source
    let result = repo
        .would_create_cycle(&proposal_a.id, &proposal_b.id)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}
