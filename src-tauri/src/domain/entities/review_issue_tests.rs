use super::*;

use super::*;

fn create_test_issue() -> ReviewIssue {
    ReviewIssue::new(
        ReviewNoteId::from_string("rn-1"),
        TaskId::from_string("task-1".to_string()),
        "Test issue".to_string(),
        IssueSeverity::Major,
    )
}

// ===== IssueStatus Tests =====

#[test]
fn issue_status_to_db_string() {
    assert_eq!(IssueStatus::Open.to_db_string(), "open");
    assert_eq!(IssueStatus::InProgress.to_db_string(), "in_progress");
    assert_eq!(IssueStatus::Addressed.to_db_string(), "addressed");
    assert_eq!(IssueStatus::Verified.to_db_string(), "verified");
    assert_eq!(IssueStatus::WontFix.to_db_string(), "wontfix");
}

#[test]
fn issue_status_from_db_string() {
    assert_eq!(
        IssueStatus::from_db_string("open").unwrap(),
        IssueStatus::Open
    );
    assert_eq!(
        IssueStatus::from_db_string("in_progress").unwrap(),
        IssueStatus::InProgress
    );
    assert_eq!(
        IssueStatus::from_db_string("addressed").unwrap(),
        IssueStatus::Addressed
    );
    assert_eq!(
        IssueStatus::from_db_string("verified").unwrap(),
        IssueStatus::Verified
    );
    assert_eq!(
        IssueStatus::from_db_string("wontfix").unwrap(),
        IssueStatus::WontFix
    );
    assert!(IssueStatus::from_db_string("invalid").is_err());
}

#[test]
fn issue_status_is_terminal() {
    assert!(!IssueStatus::Open.is_terminal());
    assert!(!IssueStatus::InProgress.is_terminal());
    assert!(!IssueStatus::Addressed.is_terminal());
    assert!(IssueStatus::Verified.is_terminal());
    assert!(IssueStatus::WontFix.is_terminal());
}

#[test]
fn issue_status_is_resolved() {
    assert!(!IssueStatus::Open.is_resolved());
    assert!(!IssueStatus::InProgress.is_resolved());
    assert!(IssueStatus::Addressed.is_resolved());
    assert!(IssueStatus::Verified.is_resolved());
    assert!(IssueStatus::WontFix.is_resolved());
}

#[test]
fn issue_status_serialization() {
    let status = IssueStatus::InProgress;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"in_progress\"");
    let parsed: IssueStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(status, parsed);
}

// ===== IssueSeverity Tests =====

#[test]
fn issue_severity_to_db_string() {
    assert_eq!(IssueSeverity::Critical.to_db_string(), "critical");
    assert_eq!(IssueSeverity::Major.to_db_string(), "major");
    assert_eq!(IssueSeverity::Minor.to_db_string(), "minor");
    assert_eq!(IssueSeverity::Suggestion.to_db_string(), "suggestion");
}

#[test]
fn issue_severity_from_db_string() {
    assert_eq!(
        IssueSeverity::from_db_string("critical").unwrap(),
        IssueSeverity::Critical
    );
    assert_eq!(
        IssueSeverity::from_db_string("major").unwrap(),
        IssueSeverity::Major
    );
    assert_eq!(
        IssueSeverity::from_db_string("minor").unwrap(),
        IssueSeverity::Minor
    );
    assert_eq!(
        IssueSeverity::from_db_string("suggestion").unwrap(),
        IssueSeverity::Suggestion
    );
    assert!(IssueSeverity::from_db_string("invalid").is_err());
}

#[test]
fn issue_severity_priority_order() {
    assert!(IssueSeverity::Critical.priority_order() < IssueSeverity::Major.priority_order());
    assert!(IssueSeverity::Major.priority_order() < IssueSeverity::Minor.priority_order());
    assert!(IssueSeverity::Minor.priority_order() < IssueSeverity::Suggestion.priority_order());
}

#[test]
fn issue_severity_serialization() {
    let severity = IssueSeverity::Critical;
    let json = serde_json::to_string(&severity).unwrap();
    assert_eq!(json, "\"critical\"");
    let parsed: IssueSeverity = serde_json::from_str(&json).unwrap();
    assert_eq!(severity, parsed);
}

// ===== IssueCategory Tests =====

#[test]
fn issue_category_to_db_string() {
    assert_eq!(IssueCategory::Bug.to_db_string(), "bug");
    assert_eq!(IssueCategory::Missing.to_db_string(), "missing");
    assert_eq!(IssueCategory::Quality.to_db_string(), "quality");
    assert_eq!(IssueCategory::Design.to_db_string(), "design");
}

#[test]
fn issue_category_from_db_string() {
    assert_eq!(
        IssueCategory::from_db_string("bug").unwrap(),
        IssueCategory::Bug
    );
    assert_eq!(
        IssueCategory::from_db_string("missing").unwrap(),
        IssueCategory::Missing
    );
    assert_eq!(
        IssueCategory::from_db_string("quality").unwrap(),
        IssueCategory::Quality
    );
    assert_eq!(
        IssueCategory::from_db_string("design").unwrap(),
        IssueCategory::Design
    );
    assert!(IssueCategory::from_db_string("invalid").is_err());
}

#[test]
fn issue_category_serialization() {
    let category = IssueCategory::Bug;
    let json = serde_json::to_string(&category).unwrap();
    assert_eq!(json, "\"bug\"");
    let parsed: IssueCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(category, parsed);
}

// ===== ReviewIssue Tests =====

#[test]
fn review_issue_new_creates_with_defaults() {
    let issue = create_test_issue();

    assert_eq!(issue.title, "Test issue");
    assert_eq!(issue.severity, IssueSeverity::Major);
    assert_eq!(issue.status, IssueStatus::Open);
    assert!(issue.step_id.is_none());
    assert!(issue.no_step_reason.is_none());
    assert!(issue.description.is_none());
    assert!(issue.category.is_none());
    assert!(issue.file_path.is_none());
    assert!(issue.line_number.is_none());
    assert!(issue.code_snippet.is_none());
    assert!(issue.resolution_notes.is_none());
    assert!(issue.addressed_in_attempt.is_none());
    assert!(issue.verified_by_review_id.is_none());
}

#[test]
fn review_issue_with_id() {
    let id = ReviewIssueId::from_string("custom-id");
    let issue = ReviewIssue::with_id(
        id.clone(),
        ReviewNoteId::from_string("rn-1"),
        TaskId::from_string("task-1".to_string()),
        "Test".to_string(),
        IssueSeverity::Minor,
    );

    assert_eq!(issue.id, id);
}

#[test]
fn review_issue_is_open() {
    let mut issue = create_test_issue();
    assert!(issue.is_open());

    issue.status = IssueStatus::InProgress;
    assert!(!issue.is_open());
}

#[test]
fn review_issue_needs_work() {
    let mut issue = create_test_issue();
    assert!(issue.needs_work());

    issue.status = IssueStatus::InProgress;
    assert!(issue.needs_work());

    issue.status = IssueStatus::Addressed;
    assert!(!issue.needs_work());
}

#[test]
fn review_issue_lifecycle_start_work() {
    let mut issue = create_test_issue();
    let original_updated = issue.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));
    issue.start_work();

    assert_eq!(issue.status, IssueStatus::InProgress);
    assert!(issue.updated_at > original_updated);
}

#[test]
fn review_issue_lifecycle_mark_addressed() {
    let mut issue = create_test_issue();
    issue.start_work();

    issue.mark_addressed(Some("Fixed the bug".to_string()), 2);

    assert_eq!(issue.status, IssueStatus::Addressed);
    assert_eq!(issue.resolution_notes, Some("Fixed the bug".to_string()));
    assert_eq!(issue.addressed_in_attempt, Some(2));
}

#[test]
fn review_issue_lifecycle_verify() {
    let mut issue = create_test_issue();
    issue.start_work();
    issue.mark_addressed(Some("Fixed".to_string()), 1);

    let verifying_review = ReviewNoteId::from_string("rn-2");
    issue.verify(verifying_review.clone());

    assert_eq!(issue.status, IssueStatus::Verified);
    assert_eq!(issue.verified_by_review_id, Some(verifying_review));
    assert!(issue.is_terminal());
}

#[test]
fn review_issue_lifecycle_reopen() {
    let mut issue = create_test_issue();
    issue.start_work();
    issue.mark_addressed(Some("Fixed".to_string()), 1);

    issue.reopen(Some("Not actually fixed".to_string()));

    assert_eq!(issue.status, IssueStatus::Open);
    assert!(issue
        .resolution_notes
        .as_ref()
        .unwrap()
        .contains("Reopened"));
    assert!(issue.verified_by_review_id.is_none());
}

#[test]
fn review_issue_lifecycle_wont_fix() {
    let mut issue = create_test_issue();

    issue.wont_fix("Not in scope for this task".to_string());

    assert_eq!(issue.status, IssueStatus::WontFix);
    assert_eq!(
        issue.resolution_notes,
        Some("Not in scope for this task".to_string())
    );
    assert!(issue.is_terminal());
}

#[test]
fn review_issue_serialization() {
    let mut issue = create_test_issue();
    issue.description = Some("Detailed description".to_string());
    issue.category = Some(IssueCategory::Bug);
    issue.file_path = Some("src/main.rs".to_string());
    issue.line_number = Some(42);

    let json = serde_json::to_string(&issue).unwrap();
    let parsed: ReviewIssue = serde_json::from_str(&json).unwrap();

    assert_eq!(issue.id, parsed.id);
    assert_eq!(issue.title, parsed.title);
    assert_eq!(issue.severity, parsed.severity);
    assert_eq!(issue.status, parsed.status);
    assert_eq!(issue.description, parsed.description);
    assert_eq!(issue.category, parsed.category);
    assert_eq!(issue.file_path, parsed.file_path);
    assert_eq!(issue.line_number, parsed.line_number);
}

// ===== IssueProgressSummary Tests =====

#[test]
fn issue_progress_summary_from_empty() {
    let task_id = TaskId::from_string("task-1".to_string());
    let summary = IssueProgressSummary::from_issues(&task_id, &[]);

    assert_eq!(summary.total, 0);
    assert_eq!(summary.open, 0);
    assert_eq!(summary.percent_resolved, 0.0);
}

#[test]
fn issue_progress_summary_calculates_correctly() {
    let task_id = TaskId::from_string("task-1".to_string());
    let review_id = ReviewNoteId::from_string("rn-1");

    let mut issues = vec![
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Issue 1".to_string(),
            IssueSeverity::Critical,
        ),
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Issue 2".to_string(),
            IssueSeverity::Major,
        ),
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Issue 3".to_string(),
            IssueSeverity::Minor,
        ),
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Issue 4".to_string(),
            IssueSeverity::Suggestion,
        ),
    ];

    issues[0].status = IssueStatus::Open;
    issues[1].status = IssueStatus::InProgress;
    issues[2].status = IssueStatus::Addressed;
    issues[3].status = IssueStatus::Verified;

    let summary = IssueProgressSummary::from_issues(&task_id, &issues);

    assert_eq!(summary.total, 4);
    assert_eq!(summary.open, 1);
    assert_eq!(summary.in_progress, 1);
    assert_eq!(summary.addressed, 1);
    assert_eq!(summary.verified, 1);
    assert_eq!(summary.wontfix, 0);
    assert_eq!(summary.percent_resolved, 50.0); // 2 resolved out of 4
}

#[test]
fn issue_progress_summary_severity_breakdown() {
    let task_id = TaskId::from_string("task-1".to_string());
    let review_id = ReviewNoteId::from_string("rn-1");

    let mut issues = vec![
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Critical 1".to_string(),
            IssueSeverity::Critical,
        ),
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Critical 2".to_string(),
            IssueSeverity::Critical,
        ),
        ReviewIssue::new(
            review_id.clone(),
            task_id.clone(),
            "Major 1".to_string(),
            IssueSeverity::Major,
        ),
    ];

    issues[0].status = IssueStatus::Open;
    issues[1].status = IssueStatus::Verified;
    issues[2].status = IssueStatus::Addressed;

    let summary = IssueProgressSummary::from_issues(&task_id, &issues);

    assert_eq!(summary.by_severity.critical.total, 2);
    assert_eq!(summary.by_severity.critical.open, 1);
    assert_eq!(summary.by_severity.critical.resolved, 1);
    assert_eq!(summary.by_severity.major.total, 1);
    assert_eq!(summary.by_severity.major.open, 0);
    assert_eq!(summary.by_severity.major.resolved, 1);
    assert_eq!(summary.by_severity.minor.total, 0);
    assert_eq!(summary.by_severity.suggestion.total, 0);
}
