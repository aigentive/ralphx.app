use super::*;
use crate::domain::entities::{ProjectId, Task};
use crate::infrastructure::sqlite::migrations::run_migrations;
use crate::infrastructure::sqlite::SqliteReviewIssueRepository;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

fn create_test_project(conn: &Connection) -> ProjectId {
    let project_id = ProjectId::new();
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            project_id.as_str(),
            "Test Project",
            "/tmp/test",
            "local",
            chrono::Utc::now().to_rfc3339(),
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .unwrap();
    project_id
}

fn create_test_task(conn: &Connection, project_id: &ProjectId) -> TaskId {
    let task = Task::new(project_id.clone(), "Test Task".to_string());
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        rusqlite::params![
            task.id.as_str(),
            task.project_id.as_str(),
            task.category.to_string(),
            task.title,
            task.description,
            task.priority,
            task.internal_status.as_str(),
            task.needs_review_point,
            task.source_proposal_id.as_ref().map(|id| id.as_str()),
            task.plan_artifact_id.as_ref().map(|id| id.as_str()),
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.started_at.map(|dt| dt.to_rfc3339()),
            task.completed_at.map(|dt| dt.to_rfc3339()),
            task.archived_at.map(|dt| dt.to_rfc3339()),
        ],
    )
    .unwrap();
    task.id
}

fn create_test_review_note(conn: &Connection, task_id: &TaskId) -> ReviewNoteId {
    let review_note_id = ReviewNoteId::new();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO review_notes (id, task_id, reviewer, outcome, summary, notes, issues, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            review_note_id.as_str(),
            task_id.as_str(),
            "ai",
            "needs_changes",
            "Test summary",
            None::<String>,
            "[]",
            now,
        ],
    )
    .unwrap();
    review_note_id
}

fn create_service(conn: Connection) -> ReviewIssueService<SqliteReviewIssueRepository> {
    let repo = SqliteReviewIssueRepository::new(conn);
    ReviewIssueService::new(Arc::new(repo))
}

fn create_test_input(title: &str) -> CreateIssueInput {
    CreateIssueInput {
        title: title.to_string(),
        description: Some("Test description".to_string()),
        severity: IssueSeverity::Major,
        category: Some(IssueCategory::Bug),
        step_id: None,
        no_step_reason: Some("Cross-cutting concern".to_string()),
        file_path: Some("src/main.rs".to_string()),
        line_number: Some(42),
        code_snippet: None,
    }
}

// ===== CreateIssueInput Validation Tests =====

#[test]
fn test_create_issue_input_validation_success() {
    let input = create_test_input("Test issue");
    assert!(input.validate().is_ok());
}

#[test]
fn test_create_issue_input_validation_with_step_id() {
    let input = CreateIssueInput {
        title: "Test issue".to_string(),
        description: None,
        severity: IssueSeverity::Minor,
        category: None,
        step_id: Some(TaskStepId::new()),
        no_step_reason: None,
        file_path: None,
        line_number: None,
        code_snippet: None,
    };
    assert!(input.validate().is_ok());
}

#[test]
fn test_create_issue_input_validation_missing_step_and_reason() {
    let input = CreateIssueInput {
        title: "Test issue".to_string(),
        description: None,
        severity: IssueSeverity::Minor,
        category: None,
        step_id: None,
        no_step_reason: None,
        file_path: None,
        line_number: None,
        code_snippet: None,
    };
    assert!(input.validate().is_err());
    assert!(input
        .validate()
        .unwrap_err()
        .contains("step_id or no_step_reason"));
}

#[test]
fn test_create_issue_input_validation_empty_title() {
    let mut input = create_test_input("");
    input.title = "   ".to_string();
    assert!(input.validate().is_err());
    assert!(input.validate().unwrap_err().contains("title"));
}

// ===== Service Tests =====

#[tokio::test]
async fn test_create_issues_from_review() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Issue 1"), create_test_input("Issue 2")];

    let issues = service
        .create_issues_from_review(review_note_id, task_id.clone(), inputs)
        .await
        .unwrap();

    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].title, "Issue 1");
    assert_eq!(issues[1].title, "Issue 2");
    assert!(issues.iter().all(|i| i.status == IssueStatus::Open));

    // Verify they're in the database
    let all_issues = service.get_issues_by_task(&task_id).await.unwrap();
    assert_eq!(all_issues.len(), 2);
}

#[tokio::test]
async fn test_create_issues_validation_failure() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    // Second input is missing step_id and no_step_reason
    let inputs = vec![
        create_test_input("Valid issue"),
        CreateIssueInput {
            title: "Invalid issue".to_string(),
            description: None,
            severity: IssueSeverity::Minor,
            category: None,
            step_id: None,
            no_step_reason: None,
            file_path: None,
            line_number: None,
            code_snippet: None,
        },
    ];

    let result = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Issue 2"));
}

#[tokio::test]
async fn test_mark_issue_in_progress() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;
    let updated = service.mark_issue_in_progress(issue_id).await.unwrap();

    assert_eq!(updated.status, IssueStatus::InProgress);
}

#[tokio::test]
async fn test_mark_issue_in_progress_wrong_status() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Mark as addressed first
    service
        .mark_issue_addressed(issue_id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();

    // Now try to mark as in_progress - should fail
    let result = service.mark_issue_in_progress(issue_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mark_issue_addressed() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Can mark addressed directly from open
    let updated = service
        .mark_issue_addressed(issue_id, Some("Fixed the bug".to_string()), 1)
        .await
        .unwrap();

    assert_eq!(updated.status, IssueStatus::Addressed);
    assert_eq!(updated.resolution_notes, Some("Fixed the bug".to_string()));
    assert_eq!(updated.addressed_in_attempt, Some(1));
}

#[tokio::test]
async fn test_mark_issue_addressed_from_in_progress() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // First mark as in_progress
    service.mark_issue_in_progress(issue_id).await.unwrap();

    // Then mark as addressed
    let updated = service
        .mark_issue_addressed(issue_id, Some("Done".to_string()), 2)
        .await
        .unwrap();

    assert_eq!(updated.status, IssueStatus::Addressed);
    assert_eq!(updated.addressed_in_attempt, Some(2));
}

#[tokio::test]
async fn test_verify_issue() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    // Create a second review note for verification (FK constraint)
    let verifying_review_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Mark as addressed first
    service
        .mark_issue_addressed(issue_id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();

    // Verify it
    let verified = service
        .verify_issue(issue_id, verifying_review_id.clone())
        .await
        .unwrap();

    assert_eq!(verified.status, IssueStatus::Verified);
    assert_eq!(verified.verified_by_review_id, Some(verifying_review_id));
    assert!(verified.is_terminal());
}

#[tokio::test]
async fn test_verify_issue_wrong_status() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id.clone(), task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Try to verify without addressing first - should fail
    let result = service.verify_issue(issue_id, review_note_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reopen_issue() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Mark as addressed
    service
        .mark_issue_addressed(issue_id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();

    // Reopen it
    let reopened = service
        .reopen_issue(issue_id, Some("Not actually fixed".to_string()))
        .await
        .unwrap();

    assert_eq!(reopened.status, IssueStatus::Open);
    assert!(reopened
        .resolution_notes
        .as_ref()
        .unwrap()
        .contains("Reopened"));
}

#[tokio::test]
async fn test_reopen_issue_wrong_status() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Try to reopen an open issue - should fail
    let result = service.reopen_issue(issue_id, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mark_issue_wontfix() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    let wontfix = service
        .mark_issue_wontfix(issue_id, "Not in scope".to_string())
        .await
        .unwrap();

    assert_eq!(wontfix.status, IssueStatus::WontFix);
    assert_eq!(wontfix.resolution_notes, Some("Not in scope".to_string()));
    assert!(wontfix.is_terminal());
}

#[tokio::test]
async fn test_mark_issue_wontfix_already_terminal() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    // Create a second review note for verification (FK constraint)
    let verifying_review = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![create_test_input("Test issue")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id, inputs)
        .await
        .unwrap();

    let issue_id = &issues[0].id;

    // Mark as addressed and verify
    service
        .mark_issue_addressed(issue_id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();
    service
        .verify_issue(issue_id, verifying_review)
        .await
        .unwrap();

    // Try to mark as wontfix - should fail
    let result = service
        .mark_issue_wontfix(issue_id, "Too late".to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_issue_progress() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![
        CreateIssueInput {
            title: "Critical issue".to_string(),
            severity: IssueSeverity::Critical,
            no_step_reason: Some("Cross-cutting".to_string()),
            ..create_test_input("Critical issue")
        },
        CreateIssueInput {
            title: "Major issue".to_string(),
            severity: IssueSeverity::Major,
            no_step_reason: Some("Cross-cutting".to_string()),
            ..create_test_input("Major issue")
        },
        CreateIssueInput {
            title: "Minor issue".to_string(),
            severity: IssueSeverity::Minor,
            no_step_reason: Some("Cross-cutting".to_string()),
            ..create_test_input("Minor issue")
        },
    ];

    let issues = service
        .create_issues_from_review(review_note_id, task_id.clone(), inputs)
        .await
        .unwrap();

    // Address one issue
    service
        .mark_issue_addressed(&issues[0].id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();

    let progress = service.get_issue_progress(&task_id).await.unwrap();

    assert_eq!(progress.total, 3);
    assert_eq!(progress.open, 2);
    assert_eq!(progress.addressed, 1);
    assert_eq!(progress.by_severity.critical.total, 1);
    assert_eq!(progress.by_severity.major.total, 1);
    assert_eq!(progress.by_severity.minor.total, 1);
}

#[tokio::test]
async fn test_get_open_issues_by_task() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    let inputs = vec![
        create_test_input("Issue 1"),
        create_test_input("Issue 2"),
        create_test_input("Issue 3"),
    ];

    let issues = service
        .create_issues_from_review(review_note_id, task_id.clone(), inputs)
        .await
        .unwrap();

    // Address one issue
    service
        .mark_issue_addressed(&issues[1].id, Some("Fixed".to_string()), 1)
        .await
        .unwrap();

    let open_issues = service.get_open_issues_by_task(&task_id).await.unwrap();
    assert_eq!(open_issues.len(), 2);
}

#[tokio::test]
async fn test_get_issue_not_found() {
    let conn = setup_test_db();
    let service = create_service(conn);

    let nonexistent = ReviewIssueId::new();
    let result = service.mark_issue_in_progress(&nonexistent).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_full_lifecycle() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    // Create a second review note for verification (FK constraint)
    let verifying_review = create_test_review_note(&conn, &task_id);
    let service = create_service(conn);

    // Create issue
    let inputs = vec![create_test_input("Bug to fix")];
    let issues = service
        .create_issues_from_review(review_note_id, task_id.clone(), inputs)
        .await
        .unwrap();
    let issue_id = &issues[0].id;

    // Initial state
    let issue = service.get_issue(issue_id).await.unwrap().unwrap();
    assert_eq!(issue.status, IssueStatus::Open);

    // Start work
    let issue = service.mark_issue_in_progress(issue_id).await.unwrap();
    assert_eq!(issue.status, IssueStatus::InProgress);

    // Address it
    let issue = service
        .mark_issue_addressed(issue_id, Some("Fixed in commit abc123".to_string()), 1)
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::Addressed);

    // Reopen (found it wasn't actually fixed)
    let issue = service
        .reopen_issue(issue_id, Some("Tests still failing".to_string()))
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::Open);

    // Start work again
    let issue = service.mark_issue_in_progress(issue_id).await.unwrap();
    assert_eq!(issue.status, IssueStatus::InProgress);

    // Address again
    let issue = service
        .mark_issue_addressed(issue_id, Some("Actually fixed now".to_string()), 2)
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::Addressed);
    assert_eq!(issue.addressed_in_attempt, Some(2));

    // Verify
    let issue = service
        .verify_issue(issue_id, verifying_review.clone())
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::Verified);
    assert_eq!(issue.verified_by_review_id, Some(verifying_review));
    assert!(issue.is_terminal());

    // Progress should show fully resolved
    let progress = service.get_issue_progress(&task_id).await.unwrap();
    assert_eq!(progress.total, 1);
    assert_eq!(progress.verified, 1);
    assert_eq!(progress.percent_resolved, 100.0);
}
