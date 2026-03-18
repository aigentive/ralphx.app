use super::*;
use crate::testing::SqliteTestDb;

fn setup_test_db() -> SqliteTestDb {
    SqliteTestDb::new("sqlite-review-repo")
}

fn create_test_project_and_task(db: &SqliteTestDb) -> (ProjectId, TaskId) {
    let project = db.seed_project("Test");
    let task = db.seed_task(project.id.clone(), "Test Task");
    (project.id, task.id)
}

#[tokio::test]
async fn test_create_and_get_review() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let review = Review::new(project_id, task_id, ReviewerType::Ai);
    let review_id = review.id.clone();

    repo.create(&review).await.unwrap();

    let retrieved = repo.get_by_id(&review_id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, review_id);
    assert_eq!(retrieved.reviewer_type, ReviewerType::Ai);
    assert_eq!(retrieved.status, ReviewStatus::Pending);
}

#[tokio::test]
async fn test_get_by_task_id() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    // Create two reviews for the same task
    let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);

    repo.create(&review1).await.unwrap();
    repo.create(&review2).await.unwrap();

    let reviews = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(reviews.len(), 2);
}

#[tokio::test]
async fn test_get_pending() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let mut review = Review::new(project_id.clone(), task_id, ReviewerType::Ai);
    repo.create(&review).await.unwrap();

    // Initially pending
    let pending = repo.get_pending(&project_id).await.unwrap();
    assert_eq!(pending.len(), 1);

    // Approve and update
    review.approve(Some("Good".to_string()));
    repo.update(&review).await.unwrap();

    // No longer pending
    let pending = repo.get_pending(&project_id).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_update_review() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let mut review = Review::new(project_id, task_id, ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    review.request_changes("Missing tests".to_string());
    repo.update(&review).await.unwrap();

    let retrieved = repo.get_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, ReviewStatus::ChangesRequested);
    assert_eq!(retrieved.notes, Some("Missing tests".to_string()));
    assert!(retrieved.completed_at.is_some());
}

#[tokio::test]
async fn test_delete_review() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let review = Review::new(project_id, task_id, ReviewerType::Ai);
    let review_id = review.id.clone();

    repo.create(&review).await.unwrap();
    repo.delete(&review_id).await.unwrap();

    let retrieved = repo.get_by_id(&review_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_add_and_get_action() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);

    // Create a fix task ID (no FK constraint needed for review_actions.target_task_id)
    let fix_task_id = TaskId::new();

    let repo = SqliteReviewRepository::new(db.new_connection());

    let review = Review::new(project_id, task_id, ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    let action = ReviewAction::with_target_task(
        review_id.clone(),
        ReviewActionType::CreatedFixTask,
        fix_task_id,
    );
    let action_id = action.id.clone();

    repo.add_action(&action).await.unwrap();

    let actions = repo.get_actions(&review_id).await.unwrap();
    assert_eq!(actions.len(), 1);
    assert!(actions[0].is_fix_task_action());

    let retrieved = repo.get_action_by_id(&action_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_add_and_get_note() {
    let db = setup_test_db();
    let (_project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let note1 = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Ai,
        ReviewOutcome::ChangesRequested,
        "Missing tests".to_string(),
    );
    let note2 = ReviewNote::with_notes(
        task_id.clone(),
        ReviewerType::Ai,
        ReviewOutcome::Approved,
        "Looks good now".to_string(),
    );
    let note1_id = note1.id.clone();

    repo.add_note(&note1).await.unwrap();
    repo.add_note(&note2).await.unwrap();

    let notes = repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert_eq!(notes.len(), 2);
    // Notes should be ordered by created_at
    assert_eq!(notes[0].outcome, ReviewOutcome::ChangesRequested);
    assert_eq!(notes[1].outcome, ReviewOutcome::Approved);

    let retrieved = repo.get_note_by_id(&note1_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_get_by_status() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let mut review2 = Review::new(project_id.clone(), task_id, ReviewerType::Ai);

    review2.approve(None);

    repo.create(&review1).await.unwrap();
    repo.create(&review2).await.unwrap();

    let pending = repo
        .get_by_status(&project_id, ReviewStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    let approved = repo
        .get_by_status(&project_id, ReviewStatus::Approved)
        .await
        .unwrap();
    assert_eq!(approved.len(), 1);
}

#[tokio::test]
async fn test_count_pending() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let review2 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let mut review3 = Review::new(project_id.clone(), task_id, ReviewerType::Ai);

    review3.approve(None);

    repo.create(&review1).await.unwrap();
    repo.create(&review2).await.unwrap();
    repo.create(&review3).await.unwrap();

    let count = repo.count_pending(&project_id).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_has_pending_review() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    // No review yet
    assert!(!repo.has_pending_review(&task_id).await.unwrap());

    // Create pending review
    let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
    repo.create(&review).await.unwrap();

    assert!(repo.has_pending_review(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_review_cascade_delete() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    // Add an action
    let action = ReviewAction::new(review_id.clone(), ReviewActionType::Approved);
    repo.add_action(&action).await.unwrap();

    // Delete the review - action should be cascade deleted
    repo.delete(&review_id).await.unwrap();

    let actions = repo.get_actions(&review_id).await.unwrap();
    assert_eq!(actions.len(), 0);
}

#[tokio::test]
async fn test_count_fix_actions() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    // No fix actions yet
    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 0);

    // Create a review and add fix task action
    let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    let fix_task_id = TaskId::new();
    let action = ReviewAction::with_target_task(
        review_id.clone(),
        ReviewActionType::CreatedFixTask,
        fix_task_id,
    );
    repo.add_action(&action).await.unwrap();

    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 1);

    // Add another fix task action
    let fix_task_id_2 = TaskId::new();
    let action2 = ReviewAction::with_target_task(
        review_id.clone(),
        ReviewActionType::CreatedFixTask,
        fix_task_id_2,
    );
    repo.add_action(&action2).await.unwrap();

    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);

    // Add a non-fix action (should not be counted)
    let action3 = ReviewAction::new(review_id, ReviewActionType::Approved);
    repo.add_action(&action3).await.unwrap();

    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);
}

#[tokio::test]
async fn test_get_fix_actions() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    // Create a review and add actions
    let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    let fix_task_id = TaskId::new();
    let action1 = ReviewAction::with_target_task(
        review_id.clone(),
        ReviewActionType::CreatedFixTask,
        fix_task_id.clone(),
    );
    repo.add_action(&action1).await.unwrap();

    // Add a non-fix action (should not be returned)
    let action2 = ReviewAction::new(review_id, ReviewActionType::Approved);
    repo.add_action(&action2).await.unwrap();

    let fix_actions = repo.get_fix_actions(&task_id).await.unwrap();
    assert_eq!(fix_actions.len(), 1);
    assert!(fix_actions[0].is_fix_task_action());
    assert_eq!(fix_actions[0].target_task_id, Some(fix_task_id));
}

#[tokio::test]
async fn test_count_fix_actions_across_multiple_reviews() {
    let db = setup_test_db();
    let (project_id, task_id) = create_test_project_and_task(&db);
    let repo = SqliteReviewRepository::new(db.new_connection());

    // Create first review with fix action
    let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    repo.create(&review1).await.unwrap();

    let action1 = ReviewAction::with_target_task(
        review1.id.clone(),
        ReviewActionType::CreatedFixTask,
        TaskId::new(),
    );
    repo.add_action(&action1).await.unwrap();

    // Create second review with fix action
    let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
    repo.create(&review2).await.unwrap();

    let action2 = ReviewAction::with_target_task(
        review2.id.clone(),
        ReviewActionType::CreatedFixTask,
        TaskId::new(),
    );
    repo.add_action(&action2).await.unwrap();

    // Should count both fix actions across reviews
    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 2);
}
