use super::*;
use crate::domain::entities::{IssueCategory, IssueSeverity, ProjectId, ReviewNoteId, Task};
use crate::infrastructure::sqlite::migrations::run_migrations;
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

fn create_test_issue(review_note_id: &ReviewNoteId, task_id: &TaskId) -> ReviewIssue {
    ReviewIssue::new(
        review_note_id.clone(),
        task_id.clone(),
        "Test issue".to_string(),
        IssueSeverity::Major,
    )
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let mut issue = create_test_issue(&review_note_id, &task_id);
    issue.description = Some("Test description".to_string());
    issue.category = Some(IssueCategory::Bug);
    issue.file_path = Some("src/main.rs".to_string());
    issue.line_number = Some(42);
    let issue_id = issue.id.clone();

    // Create issue
    let created = repo.create(issue).await.unwrap();
    assert_eq!(created.title, "Test issue");
    assert_eq!(created.severity, IssueSeverity::Major);
    assert_eq!(created.status, IssueStatus::Open);

    // Get by ID
    let fetched = repo.get_by_id(&issue_id).await.unwrap();
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.title, "Test issue");
    assert_eq!(fetched.description, Some("Test description".to_string()));
    assert_eq!(fetched.category, Some(IssueCategory::Bug));
    assert_eq!(fetched.file_path, Some("src/main.rs".to_string()));
    assert_eq!(fetched.line_number, Some(42));
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let conn = setup_test_db();
    let repo = SqliteReviewIssueRepository::new(conn);

    let issue_id = ReviewIssueId::new();
    let result = repo.get_by_id(&issue_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_task_id() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let issue1 = create_test_issue(&review_note_id, &task_id);
    let mut issue2 = create_test_issue(&review_note_id, &task_id);
    issue2.title = "Second issue".to_string();

    repo.create(issue1).await.unwrap();
    repo.create(issue2).await.unwrap();

    let issues = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(issues.len(), 2);
}

#[tokio::test]
async fn test_get_open_by_task_id() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let issue1 = create_test_issue(&review_note_id, &task_id);
    let mut issue2 = create_test_issue(&review_note_id, &task_id);
    issue2.title = "Addressed issue".to_string();
    issue2.status = IssueStatus::Addressed;

    let issue1_id = issue1.id.clone();
    repo.create(issue1).await.unwrap();
    repo.create(issue2).await.unwrap();

    let open_issues = repo.get_open_by_task_id(&task_id).await.unwrap();
    assert_eq!(open_issues.len(), 1);
    assert_eq!(open_issues[0].id, issue1_id);
}

#[tokio::test]
async fn test_update_status() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let issue = create_test_issue(&review_note_id, &task_id);
    let issue_id = issue.id.clone();

    repo.create(issue).await.unwrap();

    // Update status
    let updated = repo
        .update_status(
            &issue_id,
            IssueStatus::Addressed,
            Some("Fixed the bug".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(updated.status, IssueStatus::Addressed);
    assert_eq!(updated.resolution_notes, Some("Fixed the bug".to_string()));
}

#[tokio::test]
async fn test_update_full_issue() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let mut issue = create_test_issue(&review_note_id, &task_id);
    let issue_id = issue.id.clone();
    repo.create(issue.clone()).await.unwrap();

    // Update using lifecycle method
    issue.start_work();
    issue.mark_addressed(Some("Fixed".to_string()), 2);
    repo.update(&issue).await.unwrap();

    // Verify
    let fetched = repo.get_by_id(&issue_id).await.unwrap().unwrap();
    assert_eq!(fetched.status, IssueStatus::Addressed);
    assert_eq!(fetched.resolution_notes, Some("Fixed".to_string()));
    assert_eq!(fetched.addressed_in_attempt, Some(2));
}

#[tokio::test]
async fn test_bulk_create() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let issues = vec![
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.title = "Issue 1".to_string();
            i.severity = IssueSeverity::Critical;
            i
        },
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.title = "Issue 2".to_string();
            i.severity = IssueSeverity::Minor;
            i
        },
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.title = "Issue 3".to_string();
            i.severity = IssueSeverity::Suggestion;
            i
        },
    ];

    let created = repo.bulk_create(issues).await.unwrap();
    assert_eq!(created.len(), 3);

    let fetched = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(fetched.len(), 3);
}

#[tokio::test]
async fn test_bulk_create_rollback_on_error() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let issue = create_test_issue(&review_note_id, &task_id);
    let issue_id = issue.id.clone();

    // Create first issue
    repo.create(issue.clone()).await.unwrap();

    // Try to bulk create with duplicate ID (should fail and rollback)
    let issues = vec![
        issue.clone(), // Duplicate ID
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.title = "New issue".to_string();
            i
        },
    ];

    let result = repo.bulk_create(issues).await;
    assert!(result.is_err());

    // Verify only the original issue exists
    let fetched = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].id, issue_id);
}

#[tokio::test]
async fn test_get_summary() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    // Create issues with different statuses and severities
    let issues = vec![
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.severity = IssueSeverity::Critical;
            i.status = IssueStatus::Open;
            i
        },
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.severity = IssueSeverity::Major;
            i.status = IssueStatus::InProgress;
            i
        },
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.severity = IssueSeverity::Minor;
            i.status = IssueStatus::Addressed;
            i
        },
        {
            let mut i = create_test_issue(&review_note_id, &task_id);
            i.severity = IssueSeverity::Suggestion;
            i.status = IssueStatus::Verified;
            i
        },
    ];

    repo.bulk_create(issues).await.unwrap();

    let summary = repo.get_summary(&task_id).await.unwrap();
    assert_eq!(summary.total, 4);
    assert_eq!(summary.open, 1);
    assert_eq!(summary.in_progress, 1);
    assert_eq!(summary.addressed, 1);
    assert_eq!(summary.verified, 1);
    assert_eq!(summary.wontfix, 0);
    assert_eq!(summary.percent_resolved, 50.0); // 2 resolved out of 4

    // Check severity breakdown
    assert_eq!(summary.by_severity.critical.total, 1);
    assert_eq!(summary.by_severity.critical.open, 1);
    assert_eq!(summary.by_severity.major.total, 1);
    assert_eq!(summary.by_severity.minor.total, 1);
    assert_eq!(summary.by_severity.suggestion.total, 1);
}

#[tokio::test]
async fn test_get_summary_empty() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let summary = repo.get_summary(&task_id).await.unwrap();
    assert_eq!(summary.total, 0);
    assert_eq!(summary.open, 0);
    assert_eq!(summary.percent_resolved, 0.0);
}

#[tokio::test]
async fn test_issue_with_all_optional_fields() {
    let conn = setup_test_db();
    let project_id = create_test_project(&conn);
    let task_id = create_test_task(&conn, &project_id);
    let review_note_id = create_test_review_note(&conn, &task_id);
    let verifying_review = create_test_review_note(&conn, &task_id);
    let repo = SqliteReviewIssueRepository::new(conn);

    let mut issue = create_test_issue(&review_note_id, &task_id);
    issue.description = Some("Full description".to_string());
    issue.category = Some(IssueCategory::Quality);
    issue.file_path = Some("/path/to/file.rs".to_string());
    issue.line_number = Some(100);
    issue.code_snippet = Some("fn buggy_code() {}".to_string());
    issue.no_step_reason = Some("Cross-cutting concern".to_string());
    issue.resolution_notes = Some("Fixed by refactoring".to_string());
    issue.addressed_in_attempt = Some(3);
    issue.verified_by_review_id = Some(verifying_review.clone());
    issue.status = IssueStatus::Verified;

    let issue_id = issue.id.clone();
    repo.create(issue).await.unwrap();

    let fetched = repo.get_by_id(&issue_id).await.unwrap().unwrap();
    assert_eq!(fetched.description, Some("Full description".to_string()));
    assert_eq!(fetched.category, Some(IssueCategory::Quality));
    assert_eq!(fetched.file_path, Some("/path/to/file.rs".to_string()));
    assert_eq!(fetched.line_number, Some(100));
    assert_eq!(fetched.code_snippet, Some("fn buggy_code() {}".to_string()));
    assert_eq!(
        fetched.no_step_reason,
        Some("Cross-cutting concern".to_string())
    );
    assert_eq!(
        fetched.resolution_notes,
        Some("Fixed by refactoring".to_string())
    );
    assert_eq!(fetched.addressed_in_attempt, Some(3));
    assert_eq!(fetched.verified_by_review_id, Some(verifying_review));
    assert_eq!(fetched.status, IssueStatus::Verified);
}
