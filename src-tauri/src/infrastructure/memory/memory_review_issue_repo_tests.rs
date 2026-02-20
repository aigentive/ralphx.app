use super::*;
use crate::domain::entities::{IssueSeverity, ReviewNoteId};

fn create_test_issue(task_id: &TaskId, title: &str) -> ReviewIssue {
    let review_note_id = ReviewNoteId::new();
    let mut issue = ReviewIssue::new(
        review_note_id,
        task_id.clone(),
        title.to_string(),
        IssueSeverity::Major,
    );
    issue.no_step_reason = Some("Cross-cutting concern".to_string());
    issue
}

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryReviewIssueRepository::new();
    let task_id = TaskId::new();
    let issue = create_test_issue(&task_id, "Test issue");

    let created = repo.create(issue.clone()).await.unwrap();
    assert_eq!(created.title, "Test issue");

    let retrieved = repo.get_by_id(&created.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Test issue");
}

#[tokio::test]
async fn test_bulk_create() {
    let repo = MemoryReviewIssueRepository::new();
    let task_id = TaskId::new();

    let issues = vec![
        create_test_issue(&task_id, "Issue 1"),
        create_test_issue(&task_id, "Issue 2"),
    ];

    let created = repo.bulk_create(issues).await.unwrap();
    assert_eq!(created.len(), 2);

    let all = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_open_by_task_id() {
    let repo = MemoryReviewIssueRepository::new();
    let task_id = TaskId::new();

    let issue1 = create_test_issue(&task_id, "Open issue");
    let mut issue2 = create_test_issue(&task_id, "Addressed issue");
    issue2.status = IssueStatus::Addressed;

    repo.create(issue1).await.unwrap();
    repo.create(issue2).await.unwrap();

    let open = repo.get_open_by_task_id(&task_id).await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].title, "Open issue");
}

#[tokio::test]
async fn test_get_summary() {
    let repo = MemoryReviewIssueRepository::new();
    let task_id = TaskId::new();

    let issue1 = create_test_issue(&task_id, "Open issue");
    let mut issue2 = create_test_issue(&task_id, "Addressed issue");
    issue2.status = IssueStatus::Addressed;
    let mut issue3 = create_test_issue(&task_id, "Verified issue");
    issue3.status = IssueStatus::Verified;

    repo.create(issue1).await.unwrap();
    repo.create(issue2).await.unwrap();
    repo.create(issue3).await.unwrap();

    let summary = repo.get_summary(&task_id).await.unwrap();
    assert_eq!(summary.total, 3);
    assert_eq!(summary.open, 1);
    assert_eq!(summary.addressed, 1);
    assert_eq!(summary.verified, 1);
}
