// Tests for SqliteVerificationCriticResultRepository

use crate::domain::entities::verification_critic_result::CriticKind;
use crate::domain::repositories::verification_critic_result_repo::{
    SubmitCriticResultInput, VerificationCriticResultRepo,
};
use crate::error::AppError;
use crate::infrastructure::sqlite::sqlite_verification_critic_result_repo::SqliteVerificationCriticResultRepository;
use crate::testing::SqliteTestDb;

fn setup_repo() -> (SqliteTestDb, SqliteVerificationCriticResultRepository) {
    let db = SqliteTestDb::new("sqlite-verification-critic-result-repo");
    let repo = SqliteVerificationCriticResultRepository::from_shared(db.shared_conn());
    (db, repo)
}

fn make_input(
    parent_session_id: &str,
    generation: i32,
    round: i32,
    critic_kind: CriticKind,
) -> SubmitCriticResultInput {
    SubmitCriticResultInput {
        parent_session_id: parent_session_id.to_string(),
        verification_session_id: "vsid-test".to_string(),
        verification_generation: generation,
        round,
        critic_kind,
        title: "Test critic result".to_string(),
        content: r#"{"status":"complete","gaps":[]}"#.to_string(),
        artifact_type: None,
    }
}

#[tokio::test]
async fn test_submit_creates_artifact_and_result_atomically() {
    let (db, repo) = setup_repo();

    let output = repo
        .submit(make_input("session-1", 1, 1, CriticKind::Completeness))
        .await
        .expect("submit should succeed");

    assert!(!output.artifact_id.is_empty());
    assert!(!output.result_id.is_empty());

    // Verify artifact row was created
    let artifact_count: i64 = db.with_connection(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM artifacts WHERE id = ?1",
            [&output.artifact_id],
            |row| row.get(0),
        )
        .unwrap()
    });
    assert_eq!(artifact_count, 1, "artifact row should be created");

    // Verify result row was created
    let result_count: i64 = db.with_connection(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM verification_critic_results WHERE id = ?1",
            [&output.result_id],
            |row| row.get(0),
        )
        .unwrap()
    });
    assert_eq!(result_count, 1, "result row should be created");
}

#[tokio::test]
async fn test_submit_duplicate_returns_conflict() {
    let (_db, repo) = setup_repo();

    repo.submit(make_input("session-1", 1, 1, CriticKind::Feasibility))
        .await
        .expect("first submit should succeed");

    let err = repo
        .submit(make_input("session-1", 1, 1, CriticKind::Feasibility))
        .await
        .expect_err("duplicate submit should fail");

    assert!(
        matches!(err, AppError::Conflict(_)),
        "expected Conflict, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_get_round_results_returns_correct_rows() {
    let (_db, repo) = setup_repo();

    // Submit two critics for the same round
    repo.submit(make_input("session-2", 1, 1, CriticKind::Completeness))
        .await
        .expect("submit completeness");
    repo.submit(make_input("session-2", 1, 1, CriticKind::Feasibility))
        .await
        .expect("submit feasibility");
    // Submit one for a different round
    repo.submit(make_input("session-2", 1, 2, CriticKind::Completeness))
        .await
        .expect("submit round 2");

    let results = repo
        .get_round_results("session-2", 1, 1)
        .await
        .expect("get_round_results should succeed");

    assert_eq!(results.len(), 2, "should return exactly 2 results for round 1");
    let kinds: Vec<_> = results.iter().map(|r| r.critic_kind.as_str()).collect();
    assert!(kinds.contains(&"completeness"));
    assert!(kinds.contains(&"feasibility"));
}

#[tokio::test]
async fn test_get_round_results_generation_isolation() {
    let (_db, repo) = setup_repo();

    // Submit for generation 1
    repo.submit(make_input("session-3", 1, 1, CriticKind::Intent))
        .await
        .expect("submit gen 1");

    // Query generation 2 — must return empty
    let results = repo
        .get_round_results("session-3", 2, 1)
        .await
        .expect("get_round_results gen 2 should succeed");

    assert!(
        results.is_empty(),
        "generation 2 query should return no results when only gen 1 was submitted"
    );
}
