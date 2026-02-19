// Commit message tests
//
// Extracted from side_effects.rs — tests for category_to_commit_type,
// derive_commit_type, build_squash_commit_msg, and build_plan_merge_commit_msg.
//
// NOTE: These test private functions from side_effects.rs that will be promoted
// to pub(super) in a later step.

use super::helpers::*;
use super::super::commit_messages::{
    category_to_commit_type, derive_commit_type, build_squash_commit_msg,
    build_plan_merge_commit_msg,
};
use crate::domain::entities::types::IdeationSessionId;
use crate::domain::entities::TaskCategory;
use crate::domain::repositories::IdeationSessionRepository;
use crate::infrastructure::memory::{MemoryIdeationSessionRepository, MemoryTaskRepository};

// =========================================================================
// category_to_commit_type + derive_commit_type tests
// =========================================================================

#[test]
fn test_category_to_commit_type_mappings() {
    assert_eq!(category_to_commit_type(&TaskCategory::Regular), "feat");
    assert_eq!(category_to_commit_type(&TaskCategory::PlanMerge), "feat");
}

#[test]
fn test_derive_commit_type_empty_returns_feat() {
    assert_eq!(derive_commit_type(&[]), "feat");
}

#[test]
fn test_derive_commit_type_single_regular() {
    let tasks = vec![make_task_with_category(TaskCategory::Regular)];
    assert_eq!(derive_commit_type(&tasks), "feat");
}

#[test]
fn test_derive_commit_type_multiple_tasks() {
    let tasks = vec![
        make_task_with_category(TaskCategory::Regular),
        make_task_with_category(TaskCategory::Regular),
        make_task_with_category(TaskCategory::PlanMerge),
    ];
    assert_eq!(derive_commit_type(&tasks), "feat");
}

// =========================================================================
// build_squash_commit_msg (regular tasks) tests
// =========================================================================

#[test]
fn test_build_squash_commit_msg_regular_task() {
    let msg = build_squash_commit_msg(&TaskCategory::Regular, "Write tests", "ralphx/ralphx/task-xyz");
    assert_eq!(msg, "feat: ralphx/ralphx/task-xyz (Write tests)");
}

#[test]
fn test_build_squash_commit_msg_different_category() {
    let msg = build_squash_commit_msg(&TaskCategory::PlanMerge, "Fix bug", "ralphx/ralphx/task-123");
    assert_eq!(msg, "feat: ralphx/ralphx/task-123 (Fix bug)");
}

// =========================================================================
// build_plan_merge_commit_msg (async) tests
// =========================================================================

#[tokio::test]
async fn test_build_plan_merge_commit_msg_with_session_title_and_tasks() {
    let session_id = "sess-001";
    let session = make_session_with_title_for_test(session_id, "Add OAuth2 login");
    let tasks = vec![
        make_plan_task(session_id, "Add JWT token refresh endpoint"),
        make_plan_task(session_id, "Implement OAuth2 provider integration"),
        make_plan_task(session_id, "Add session expiry UI warning"),
    ];

    let task_repo = MemoryTaskRepository::with_tasks(tasks);
    let session_repo = MemoryIdeationSessionRepository::new();
    let sid = IdeationSessionId::from_string(session_id.to_string());
    session_repo.create(session).await.unwrap();

    let msg = build_plan_merge_commit_msg(
        &sid,
        "ralphx/ralphx/plan-a3b2c1d0",
        &task_repo,
        &session_repo,
    )
    .await;

    assert!(
        msg.starts_with("feat: Add OAuth2 login"),
        "Should start with derived type and session title, got: {}",
        msg
    );
    assert!(
        msg.contains("Plan branch: ralphx/ralphx/plan-a3b2c1d0"),
        "Should contain plan branch"
    );
    assert!(
        msg.contains("Tasks (3):"),
        "Should list task count, got: {}",
        msg
    );
    assert!(
        msg.contains("- Add JWT token refresh endpoint"),
        "Should list first task"
    );
}

#[tokio::test]
async fn test_build_plan_merge_commit_msg_no_session_title_falls_back_to_first_task() {
    let session_id = "sess-003";
    let session = make_session_no_title(session_id);
    let tasks = vec![make_plan_task(session_id, "Add payment gateway")];

    let task_repo = MemoryTaskRepository::with_tasks(tasks);
    let session_repo = MemoryIdeationSessionRepository::new();
    let sid = IdeationSessionId::from_string(session_id.to_string());
    session_repo.create(session).await.unwrap();

    let msg = build_plan_merge_commit_msg(
        &sid,
        "ralphx/ralphx/plan-111",
        &task_repo,
        &session_repo,
    )
    .await;

    assert!(
        msg.starts_with("feat: Add payment gateway"),
        "Should use first task title as fallback, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_build_plan_merge_commit_msg_no_session_no_tasks_uses_generic() {
    let session_id = "sess-004";
    let session = make_session_no_title(session_id);

    let task_repo = MemoryTaskRepository::new();
    let session_repo = MemoryIdeationSessionRepository::new();
    let sid = IdeationSessionId::from_string(session_id.to_string());
    session_repo.create(session).await.unwrap();

    let msg = build_plan_merge_commit_msg(
        &sid,
        "ralphx/ralphx/plan-222",
        &task_repo,
        &session_repo,
    )
    .await;

    assert!(
        msg.contains("Merge plan into main"),
        "Should use generic fallback when no title or tasks, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_build_plan_merge_commit_msg_truncates_at_20_tasks() {
    let session_id = "sess-005";
    let session = make_session_with_title_for_test(session_id, "Big refactor");
    let tasks: Vec<_> = (1..=25)
        .map(|i| make_plan_task(session_id, &format!("Task {}", i)))
        .collect();

    let task_repo = MemoryTaskRepository::with_tasks(tasks);
    let session_repo = MemoryIdeationSessionRepository::new();
    let sid = IdeationSessionId::from_string(session_id.to_string());
    session_repo.create(session).await.unwrap();

    let msg = build_plan_merge_commit_msg(
        &sid,
        "ralphx/ralphx/plan-333",
        &task_repo,
        &session_repo,
    )
    .await;

    assert!(
        msg.contains("Tasks (25):"),
        "Should show total count of 25, got: {}",
        msg
    );
    assert!(
        msg.contains("(+5 more)"),
        "Should show overflow count, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_build_plan_merge_commit_msg_user_renamed_title() {
    let session_id = "sess-006";
    let session =
        make_session_with_title_for_test(session_id, "Add OAuth2 login and JWT sessions");
    let tasks = vec![
        make_plan_task(session_id, "Add JWT token refresh endpoint"),
        make_plan_task(session_id, "Implement OAuth2 provider integration"),
    ];

    let task_repo = MemoryTaskRepository::with_tasks(tasks);
    let session_repo = MemoryIdeationSessionRepository::new();
    let sid = IdeationSessionId::from_string(session_id.to_string());
    session_repo.create(session).await.unwrap();

    let msg = build_plan_merge_commit_msg(
        &sid,
        "ralphx/ralphx/plan-444",
        &task_repo,
        &session_repo,
    )
    .await;

    assert_eq!(
        msg.lines().next().unwrap(),
        "feat: Add OAuth2 login and JWT sessions",
        "Should use user-renamed session title as subject"
    );
}
