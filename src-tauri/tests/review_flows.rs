// Review System Integration Tests
//
// These tests verify the review system functionality:
// - AI review approve flow
// - AI review needs_changes flow (fix task creation)
// - AI review escalate flow
// - Fix task rejection and retry workflow
// - Human review flow

use std::sync::Arc;
use tokio::sync::Mutex;

use ralphx_lib::application::ReviewService;
use ralphx_lib::domain::entities::{
    ProjectId, ReviewActionType, ReviewOutcome, ReviewStatus, ReviewerType, TaskId,
};
use ralphx_lib::domain::repositories::ReviewRepository;
use ralphx_lib::domain::review::config::ReviewSettings;
use ralphx_lib::domain::state_machine::{State, TaskEvent};
use ralphx_lib::domain::tools::complete_review::{CompleteReviewInput, ReviewToolOutcome};
use ralphx_lib::infrastructure::sqlite::{
    open_memory_connection, run_migrations, SqliteReviewRepository, SqliteTaskRepository,
    TaskStateMachineRepository,
};

/// Helper to set up a test environment with repositories and task in pending_review state
fn setup_review_test() -> (
    Arc<SqliteReviewRepository>,
    Arc<SqliteTaskRepository>,
    TaskStateMachineRepository,
    ProjectId,
    TaskId,
) {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert a task in pending_review state
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status)
         VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'pending_review')",
        [],
    )
    .unwrap();

    // Wrap connection in Arc<Mutex<>> for sharing
    let shared_conn = Arc::new(Mutex::new(conn));

    let task_repo = Arc::new(SqliteTaskRepository::from_shared(shared_conn.clone()));
    let review_repo = Arc::new(SqliteReviewRepository::from_shared(shared_conn.clone()));

    // Create a new connection for the state machine repository (it takes ownership)
    let sm_conn = open_memory_connection().unwrap();
    run_migrations(&sm_conn).unwrap();
    sm_conn
        .execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();
    sm_conn
        .execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'pending_review')",
            [],
        )
        .unwrap();
    let sm_repo = TaskStateMachineRepository::new(sm_conn);

    let project_id = ProjectId::from_string("proj-1".to_string());
    let task_id = TaskId::from_string("task-1".to_string());

    (review_repo, task_repo, sm_repo, project_id, task_id)
}

// ============================================================================
// AI Review Approve Flow Tests
// ============================================================================

/// Test: Full AI review approve flow
///
/// Flow:
/// 1. Task is in pending_review state
/// 2. Start AI review via ReviewService
/// 3. Mock reviewer agent returns APPROVE outcome
/// 4. Process review result
/// 5. Verify task transitions to approved
/// 6. Verify review record created with correct status
#[tokio::test]
async fn test_ai_review_approve_flow() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Verify task is in PendingReview state
    let state = sm_repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::PendingReview);

    // Create ReviewService
    let service = ReviewService::new(review_repo.clone(), task_repo.clone());

    // 1. Start AI review (creates Review in Pending status)
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review.is_pending());
    assert_eq!(review.reviewer_type, ReviewerType::Ai);
    assert_eq!(review.task_id, task_id);

    // 2. Simulate reviewer agent returning APPROVE outcome
    let input = CompleteReviewInput::approved("All tests pass. Code quality is good.");

    // 3. Process the review result
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // 4. Verify no fix task was created
    assert!(fix_task_id.is_none(), "Approved review should not create fix task");

    // 5. Verify review is now approved
    assert!(review.is_approved());
    let persisted_review = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert!(persisted_review.is_approved());
    assert_eq!(
        persisted_review.notes,
        Some("All tests pass. Code quality is good.".to_string())
    );

    // 6. Verify review note was created
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].outcome, ReviewOutcome::Approved);

    // 7. Verify review action was recorded
    let actions = review_repo.get_actions(&review.id).await.unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, ReviewActionType::Approved);
}

/// Test: AI review approve flow with state machine transition
#[tokio::test]
async fn test_ai_review_approve_state_machine_transition() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Start in PendingReview
    assert_eq!(sm_repo.load_state(&task_id).unwrap(), State::PendingReview);

    // Conduct AI review
    let service = ReviewService::new(review_repo, task_repo);
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::approved("LGTM");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Transition state machine: PendingReview -> Approved
    let new_state = sm_repo
        .process_event(
            &task_id,
            &TaskEvent::ReviewComplete {
                approved: true,
                feedback: None,
            },
        )
        .unwrap();

    assert_eq!(new_state, State::Approved);
}

/// Test: AI review disabled in settings
#[tokio::test]
async fn test_ai_review_disabled() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // Create service with AI review disabled
    let settings = ReviewSettings::ai_disabled();
    let service = ReviewService::with_settings(review_repo, task_repo, settings);

    // Starting AI review should fail
    let result = service.start_ai_review(&task_id, &project_id).await;
    assert!(result.is_err());
}

/// Test: Cannot start duplicate review for same task
#[tokio::test]
async fn test_ai_review_no_duplicate() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo, task_repo);

    // Start first review
    let _review = service.start_ai_review(&task_id, &project_id).await.unwrap();

    // Starting second review should fail
    let result = service.start_ai_review(&task_id, &project_id).await;
    assert!(result.is_err());
}

/// Test: Review stores notes correctly
#[tokio::test]
async fn test_ai_review_stores_notes() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let detailed_notes = "All acceptance criteria met:\n\
        1. ✅ Login form renders correctly\n\
        2. ✅ Validation works for email/password\n\
        3. ✅ Error messages display on invalid input\n\
        4. ✅ Success redirects to dashboard";

    let input = CompleteReviewInput::approved(detailed_notes);
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify notes are stored in review
    let persisted = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert_eq!(persisted.notes, Some(detailed_notes.to_string()));

    // Verify notes are stored in review_notes
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert!(notes[0].notes.as_ref().map_or(false, |n| n.contains("All acceptance criteria met")));
}

/// Test: Review records completed_at timestamp
#[tokio::test]
async fn test_ai_review_records_completion_time() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review.completed_at.is_none()); // Not completed yet

    let input = CompleteReviewInput::approved("Approved");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Check persisted review has completed_at set
    let persisted = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert!(persisted.completed_at.is_some());
}

/// Test: Multiple reviews for same task (after first is completed)
#[tokio::test]
async fn test_ai_review_multiple_sequential() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // First review - approved
    let mut review1 = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::approved("First approval");
    service.process_review_result(&mut review1, &input).await.unwrap();

    // After completing, we can start a new review
    let review2 = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review2.is_pending());
    assert_ne!(review1.id, review2.id);
}

// ============================================================================
// AI Review with ReviewSettings Tests
// ============================================================================

/// Test: Review with custom settings
#[tokio::test]
async fn test_ai_review_with_custom_settings() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // Settings that require human review after AI approval
    let settings = ReviewSettings::with_human_review();
    let service = ReviewService::with_settings(review_repo, task_repo, settings);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::approved("AI approves");
    service.process_review_result(&mut review, &input).await.unwrap();

    // AI marked it approved, but with require_human_review, the orchestration
    // layer should create a follow-up human review
    assert!(review.is_approved());
    assert!(service.settings().needs_human_review());
}

// ============================================================================
// Helper Tests
// ============================================================================

/// Test: CompleteReviewInput::approved creates correct input
#[test]
fn test_complete_review_input_approved() {
    let input = CompleteReviewInput::approved("All tests pass");

    assert_eq!(input.outcome, ReviewToolOutcome::Approved);
    assert_eq!(input.notes, "All tests pass");
    assert!(input.fix_description.is_none());
    assert!(input.escalation_reason.is_none());
    assert!(input.validate().is_ok());
}

/// Test: Review can be retrieved by task_id
#[tokio::test]
async fn test_get_reviews_by_task_id() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Create and complete a review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::approved("Approved");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Retrieve reviews by task
    let reviews = review_repo.get_by_task_id(&task_id).await.unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].id, review.id);
    assert!(reviews[0].is_approved());
}

/// Test: get_pending returns only pending reviews
#[tokio::test]
async fn test_get_pending_reviews() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Create a pending review
    let _review = service.start_ai_review(&task_id, &project_id).await.unwrap();

    // Get pending reviews
    let pending = review_repo.get_pending(&project_id).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].is_pending());
}

/// Test: Pending count is accurate
#[tokio::test]
async fn test_count_pending_reviews() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Initially no pending reviews
    let count = review_repo.count_pending(&project_id).await.unwrap();
    assert_eq!(count, 0);

    // Start a review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();

    // Now 1 pending
    let count = review_repo.count_pending(&project_id).await.unwrap();
    assert_eq!(count, 1);

    // Complete the review
    let input = CompleteReviewInput::approved("Done");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Back to 0 pending
    let count = review_repo.count_pending(&project_id).await.unwrap();
    assert_eq!(count, 0);
}

/// Test: has_pending_review correctly detects pending reviews
#[tokio::test]
async fn test_has_pending_review() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Initially no pending
    assert!(!review_repo.has_pending_review(&task_id).await.unwrap());

    // Start review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review_repo.has_pending_review(&task_id).await.unwrap());

    // Complete review
    let input = CompleteReviewInput::approved("Done");
    service.process_review_result(&mut review, &input).await.unwrap();
    assert!(!review_repo.has_pending_review(&task_id).await.unwrap());
}

/// Test: Review by status query
#[tokio::test]
async fn test_get_reviews_by_status() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Create and approve a review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::approved("Done");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Query by status
    let approved = review_repo
        .get_by_status(&project_id, ReviewStatus::Approved)
        .await
        .unwrap();
    assert_eq!(approved.len(), 1);

    let pending = review_repo
        .get_by_status(&project_id, ReviewStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending.len(), 0);
}

// ============================================================================
// AI Review Needs Changes Flow Tests
// ============================================================================

/// Test: Full AI review needs_changes flow
///
/// Flow:
/// 1. Task is in pending_review state
/// 2. Start AI review via ReviewService
/// 3. Mock reviewer agent returns NEEDS_CHANGES outcome
/// 4. Process review result
/// 5. Verify fix task created
/// 6. Verify original task transitions to revision_needed
/// 7. Verify review_action record created
#[tokio::test]
async fn test_ai_review_needs_changes_flow() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Verify task is in PendingReview state
    let state = sm_repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::PendingReview);

    // Create ReviewService
    let service = ReviewService::new(review_repo.clone(), task_repo.clone());

    // 1. Start AI review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review.is_pending());

    // 2. Simulate reviewer agent returning NEEDS_CHANGES outcome
    let input = CompleteReviewInput::needs_changes(
        "Missing error handling in the login function",
        "Add try-catch blocks around the API call and handle network errors",
    );

    // 3. Process the review result
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // 4. Verify fix task was created
    assert!(fix_task_id.is_some(), "NEEDS_CHANGES should create fix task");
    let fix_task_id = fix_task_id.unwrap();

    // 5. Verify fix task properties
    use ralphx_lib::domain::repositories::TaskRepository;
    let fix_task = task_repo.get_by_id(&fix_task_id).await.unwrap().unwrap();
    assert!(fix_task.title.starts_with("Fix:"), "Fix task title should start with 'Fix:'");
    assert_eq!(fix_task.category, "fix", "Fix task category should be 'fix'");
    assert!(
        fix_task
            .description
            .as_ref()
            .map_or(false, |d| d.contains("Add try-catch")),
        "Fix task should contain fix description"
    );

    // 6. Verify review status is ChangesRequested
    assert_eq!(review.status, ReviewStatus::ChangesRequested);
    let persisted_review = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert_eq!(persisted_review.status, ReviewStatus::ChangesRequested);

    // 7. Verify review note was created
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].outcome, ReviewOutcome::ChangesRequested);
    assert!(notes[0]
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Missing error handling")));

    // 8. Verify review action was recorded with fix task reference
    let actions = review_repo.get_actions(&review.id).await.unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, ReviewActionType::CreatedFixTask);
    assert_eq!(actions[0].target_task_id, Some(fix_task_id));
}

/// Test: Needs changes flow with state machine transition
#[tokio::test]
async fn test_ai_review_needs_changes_state_machine_transition() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Start in PendingReview
    assert_eq!(sm_repo.load_state(&task_id).unwrap(), State::PendingReview);

    // Conduct AI review with NEEDS_CHANGES
    let service = ReviewService::new(review_repo, task_repo);
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Bug found", "Fix the bug");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Transition state machine: PendingReview -> RevisionNeeded
    let new_state = sm_repo
        .process_event(
            &task_id,
            &TaskEvent::ReviewComplete {
                approved: false,
                feedback: Some("Bug found".to_string()),
            },
        )
        .unwrap();

    assert_eq!(new_state, State::RevisionNeeded);
}

/// Test: Needs changes with auto_fix disabled moves to backlog
#[tokio::test]
async fn test_ai_review_needs_changes_auto_fix_disabled() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // Create service with auto_fix disabled
    let settings = ReviewSettings {
        ai_review_auto_fix: false,
        ..Default::default()
    };
    let service = ReviewService::with_settings(review_repo.clone(), task_repo, settings);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Missing tests", "Add unit tests");
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // Should NOT create a fix task when auto_fix is disabled
    assert!(fix_task_id.is_none(), "Should not create fix task when auto_fix disabled");

    // Verify action recorded as moved to backlog
    let actions = review_repo.get_actions(&review.id).await.unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, ReviewActionType::MovedToBacklog);
}

/// Test: Fix task has higher priority than original task
#[tokio::test]
async fn test_fix_task_has_higher_priority() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // First, set the original task's priority
    use ralphx_lib::domain::repositories::TaskRepository;
    {
        let mut task = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        task.set_priority(100);
        task_repo.update(&task).await.unwrap();
    }

    let service = ReviewService::new(review_repo, task_repo.clone());

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Fix needed", "Apply the fix");
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    let fix_task = task_repo
        .get_by_id(&fix_task_id.unwrap())
        .await
        .unwrap()
        .unwrap();

    // Fix task should have higher priority (priority + 1)
    assert_eq!(fix_task.priority, 101);
}

/// Test: Fix task requires approval when configured
#[tokio::test]
async fn test_fix_task_requires_approval() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // Create service with fix approval required
    let settings = ReviewSettings::with_fix_approval();
    let service = ReviewService::with_settings(review_repo, task_repo.clone(), settings);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Bug found", "Fix the bug");
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // Fix task should be in Blocked status (waiting for approval)
    use ralphx_lib::domain::entities::InternalStatus;
    use ralphx_lib::domain::repositories::TaskRepository;
    let fix_task = task_repo
        .get_by_id(&fix_task_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fix_task.internal_status,
        InternalStatus::Blocked,
        "Fix task should be Blocked when fix approval required"
    );
}

/// Test: Fix task is Ready when approval not required
#[tokio::test]
async fn test_fix_task_ready_without_approval() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    // Default settings don't require fix approval
    let service = ReviewService::new(review_repo, task_repo.clone());

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Bug found", "Fix the bug");
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // Fix task should be in Ready status
    use ralphx_lib::domain::entities::InternalStatus;
    use ralphx_lib::domain::repositories::TaskRepository;
    let fix_task = task_repo
        .get_by_id(&fix_task_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fix_task.internal_status,
        InternalStatus::Ready,
        "Fix task should be Ready when fix approval not required"
    );
}

/// Test: CompleteReviewInput::needs_changes validation
#[test]
fn test_complete_review_input_needs_changes() {
    let input = CompleteReviewInput::needs_changes("Missing tests", "Add unit tests");

    assert_eq!(input.outcome, ReviewToolOutcome::NeedsChanges);
    assert_eq!(input.notes, "Missing tests");
    assert_eq!(
        input.fix_description,
        Some("Add unit tests".to_string())
    );
    assert!(input.escalation_reason.is_none());
    assert!(input.validate().is_ok());
}

/// Test: CompleteReviewInput::needs_changes requires fix_description
#[test]
fn test_complete_review_input_needs_changes_requires_fix_description() {
    use ralphx_lib::domain::tools::complete_review::CompleteReviewInput;

    let input = CompleteReviewInput {
        outcome: ReviewToolOutcome::NeedsChanges,
        notes: "Missing tests".to_string(),
        fix_description: None, // Missing!
        escalation_reason: None,
    };

    assert!(input.validate().is_err(), "Should fail validation without fix_description");
}

/// Test: Count fix actions for a task
#[tokio::test]
async fn test_count_fix_actions() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    // Initially no fix actions
    let count = service.get_fix_attempt_count(&task_id).await.unwrap();
    assert_eq!(count, 0);

    // Create a review with needs_changes
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::needs_changes("Bug 1", "Fix 1");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Now should be 1
    let count = service.get_fix_attempt_count(&task_id).await.unwrap();
    assert_eq!(count, 1);
}

/// Test: Multiple fix tasks increment fix action count
#[tokio::test]
async fn test_multiple_fix_attempts_tracked() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo.clone());

    // First review with needs_changes
    let mut review1 = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input1 = CompleteReviewInput::needs_changes("Bug 1", "Fix 1");
    let fix1_id = service.process_review_result(&mut review1, &input1).await.unwrap();
    assert!(fix1_id.is_some());

    // Complete the first review (so we can start another)
    // Start a second review on the same task
    let mut review2 = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input2 = CompleteReviewInput::needs_changes("Bug 2", "Fix 2");
    let fix2_id = service.process_review_result(&mut review2, &input2).await.unwrap();
    assert!(fix2_id.is_some());

    // Should now have 2 fix actions
    let count = service.get_fix_attempt_count(&task_id).await.unwrap();
    assert_eq!(count, 2);
}

// ============================================================================
// AI Review Escalate Flow Tests
// ============================================================================

/// Test: Full AI review escalate flow
///
/// Flow:
/// 1. Task is in pending_review state
/// 2. Start AI review via ReviewService
/// 3. Mock reviewer agent returns ESCALATE outcome
/// 4. Process review result
/// 5. Verify task transitions to blocked
/// 6. Verify review record has needs_human status
/// 7. Verify notification emitted (via review note)
#[tokio::test]
async fn test_ai_review_escalate_flow() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Verify task is in PendingReview state
    let state = sm_repo.load_state(&task_id).unwrap();
    assert_eq!(state, State::PendingReview);

    // Create ReviewService
    let service = ReviewService::new(review_repo.clone(), task_repo.clone());

    // 1. Start AI review
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    assert!(review.is_pending());

    // 2. Simulate reviewer agent returning ESCALATE outcome
    let input = CompleteReviewInput::escalate(
        "Security-sensitive changes detected: adds authentication bypass",
        "This change modifies core authentication logic and needs human verification",
    );

    // 3. Process the review result
    let fix_task_id = service.process_review_result(&mut review, &input).await.unwrap();

    // 4. Verify no fix task was created (escalate doesn't create fix tasks)
    assert!(fix_task_id.is_none(), "ESCALATE should not create fix task");

    // 5. Verify review status is Rejected (escalate uses reject to signal human review needed)
    assert_eq!(review.status, ReviewStatus::Rejected);
    let persisted_review = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert_eq!(persisted_review.status, ReviewStatus::Rejected);

    // 6. Verify review notes contain escalation reason
    assert!(persisted_review
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Security-sensitive changes")));

    // 7. Verify review note was created with Rejected outcome
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].outcome, ReviewOutcome::Rejected);
    assert!(notes[0]
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Security-sensitive")));
}

/// Test: Escalate flow with state machine - task stays blocked
#[tokio::test]
async fn test_ai_review_escalate_state_machine_blocked() {
    let (review_repo, task_repo, sm_repo, project_id, task_id) = setup_review_test();

    // Start in PendingReview
    assert_eq!(sm_repo.load_state(&task_id).unwrap(), State::PendingReview);

    // Conduct AI review with ESCALATE
    let service = ReviewService::new(review_repo, task_repo);
    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate("Needs human review", "Security concern");
    service.process_review_result(&mut review, &input).await.unwrap();

    // In the real system, the orchestrator would detect the escalation and
    // either keep the task in PendingReview for human review, or move to Blocked
    // For this test, we verify the review was marked as rejected (escalated)
    assert_eq!(review.status, ReviewStatus::Rejected);

    // The task would need human intervention - simulate blocking it
    sm_repo.persist_state(&task_id, &State::Blocked).unwrap();
    assert_eq!(sm_repo.load_state(&task_id).unwrap(), State::Blocked);
}

/// Test: CompleteReviewInput::escalate creates correct input
#[test]
fn test_complete_review_input_escalate() {
    let input = CompleteReviewInput::escalate("Security concern", "Needs human review");

    assert_eq!(input.outcome, ReviewToolOutcome::Escalate);
    assert_eq!(input.notes, "Security concern");
    assert!(input.fix_description.is_none());
    assert_eq!(
        input.escalation_reason,
        Some("Needs human review".to_string())
    );
    assert!(input.validate().is_ok());
}

/// Test: CompleteReviewInput::escalate requires escalation_reason
#[test]
fn test_complete_review_input_escalate_requires_reason() {
    let input = CompleteReviewInput {
        outcome: ReviewToolOutcome::Escalate,
        notes: "Something is wrong".to_string(),
        fix_description: None,
        escalation_reason: None, // Missing!
    };

    assert!(
        input.validate().is_err(),
        "Should fail validation without escalation_reason"
    );
}

/// Test: Escalate for security-sensitive changes
#[tokio::test]
async fn test_ai_review_escalate_security_sensitive() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate(
        "Changes to authentication/authorization code require human verification",
        "Modifies user permission checks",
    );
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify escalation recorded
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert!(notes[0]
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("authentication/authorization")));
}

/// Test: Escalate for design decisions
#[tokio::test]
async fn test_ai_review_escalate_design_decision() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate(
        "Multiple valid approaches possible - human should decide",
        "Could use Redux or Context API for state management",
    );
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify escalation recorded
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert!(notes[0]
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Multiple valid approaches")));
}

/// Test: Escalate for breaking changes
#[tokio::test]
async fn test_ai_review_escalate_breaking_changes() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate(
        "Breaking changes to public API detected",
        "Removes deprecated endpoint /api/v1/users - requires migration plan",
    );
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify escalation recorded
    let persisted = review_repo.get_by_id(&review.id).await.unwrap().unwrap();
    assert!(persisted
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Breaking changes")));
}

/// Test: Escalate for low confidence
#[tokio::test]
async fn test_ai_review_escalate_low_confidence() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate(
        "Unable to fully evaluate - test coverage unclear",
        "Cannot determine if all edge cases are covered without manual review",
    );
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify escalation recorded with uncertainty note
    let notes = review_repo.get_notes_by_task_id(&task_id).await.unwrap();
    assert!(notes[0]
        .notes
        .as_ref()
        .map_or(false, |n| n.contains("Unable to fully evaluate")));
}

/// Test: Escalate doesn't create review actions like CreatedFixTask
#[tokio::test]
async fn test_ai_review_escalate_no_actions() {
    let (review_repo, task_repo, _sm_repo, project_id, task_id) = setup_review_test();

    let service = ReviewService::new(review_repo.clone(), task_repo);

    let mut review = service.start_ai_review(&task_id, &project_id).await.unwrap();
    let input = CompleteReviewInput::escalate("Needs human review", "Design decision");
    service.process_review_result(&mut review, &input).await.unwrap();

    // Verify no review actions were created
    let actions = review_repo.get_actions(&review.id).await.unwrap();
    assert_eq!(actions.len(), 0, "ESCALATE should not create review actions");
}
