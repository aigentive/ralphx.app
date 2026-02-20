use super::*;
use crate::domain::entities::{ReviewOutcome, ReviewerType};

#[tokio::test]
async fn test_create_and_get_review() {
    let repo = MemoryReviewRepository::new();
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());
    let review = Review::new(project_id, task_id, ReviewerType::Ai);
    let review_id = review.id.clone();

    repo.create(&review).await.unwrap();

    let retrieved = repo.get_by_id(&review_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, review_id);
}

#[tokio::test]
async fn test_get_by_task_id() {
    let repo = MemoryReviewRepository::new();
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    let review1 = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    let review2 = Review::new(project_id, task_id.clone(), ReviewerType::Human);

    repo.create(&review1).await.unwrap();
    repo.create(&review2).await.unwrap();

    let reviews = repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(reviews.len(), 2);
}

#[tokio::test]
async fn test_get_pending() {
    let repo = MemoryReviewRepository::new();
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    let mut review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    repo.create(&review).await.unwrap();

    let pending = repo.get_pending(&project_id).await.unwrap();
    assert_eq!(pending.len(), 1);

    review.approve(Some("Good".to_string()));
    repo.update(&review).await.unwrap();

    let pending = repo.get_pending(&project_id).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_add_and_get_action() {
    let repo = MemoryReviewRepository::new();
    let review_id = ReviewId::from_string("rev-1");
    let fix_task_id = TaskId::from_string("fix-1".to_string());

    let action = ReviewAction::with_target_task(
        review_id.clone(),
        ReviewActionType::CreatedFixTask,
        fix_task_id,
    );
    let action_id = action.id.clone();

    repo.add_action(&action).await.unwrap();

    let actions = repo.get_actions(&review_id).await.unwrap();
    assert_eq!(actions.len(), 1);

    let retrieved = repo.get_action_by_id(&action_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_add_and_get_note() {
    let repo = MemoryReviewRepository::new();
    let task_id = TaskId::from_string("task-1".to_string());

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

    let retrieved = repo.get_note_by_id(&note1_id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_count_and_has_pending() {
    let repo = MemoryReviewRepository::new();
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    assert_eq!(repo.count_pending(&project_id).await.unwrap(), 0);
    assert!(!repo.has_pending_review(&task_id).await.unwrap());

    let review = Review::new(project_id.clone(), task_id.clone(), ReviewerType::Ai);
    repo.create(&review).await.unwrap();

    assert_eq!(repo.count_pending(&project_id).await.unwrap(), 1);
    assert!(repo.has_pending_review(&task_id).await.unwrap());
}

#[tokio::test]
async fn test_count_fix_actions() {
    let repo = MemoryReviewRepository::new();
    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    let review = Review::new(project_id, task_id.clone(), ReviewerType::Ai);
    let review_id = review.id.clone();
    repo.create(&review).await.unwrap();

    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 0);

    let fix_task_id = TaskId::from_string("fix-1".to_string());
    let action = ReviewAction::with_target_task(
        review_id,
        ReviewActionType::CreatedFixTask,
        fix_task_id,
    );
    repo.add_action(&action).await.unwrap();

    assert_eq!(repo.count_fix_actions(&task_id).await.unwrap(), 1);
}
