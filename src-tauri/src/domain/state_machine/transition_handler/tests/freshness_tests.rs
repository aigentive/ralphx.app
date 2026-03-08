// Tests for ensure_branches_fresh() — branch freshness check orchestration.
//
// Covers: config toggle, skip window, plan check result mapping, source check
// result mapping, retry counting, and dual-conflict sequential scenarios.

use super::helpers::*;
use super::super::freshness::{ensure_branches_fresh, FreshnessAction};
use crate::domain::entities::{Project, ProjectId, Task};
use crate::infrastructure::agents::claude::ReconciliationConfig;
use chrono::Utc;

// ==================
// Helpers
// ==================

/// Create a Project entity pointing at a repo path.
fn make_test_project(repo_path: &str) -> Project {
    let mut project = Project::new("test-project".to_string(), repo_path.to_string());
    project.base_branch = Some("main".to_string());
    project
}

/// Build a minimal Task for freshness tests.
///
/// `task_branch` — the task's branch (source branch for freshness checks).
/// `metadata` — JSON metadata to embed (freshness state lives here).
fn make_test_task(task_branch: Option<&str>, metadata: Option<serde_json::Value>) -> Task {
    let mut t = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        "Test task".into(),
    );
    t.task_branch = task_branch.map(|s| s.to_string());
    t.metadata = metadata.map(|v| v.to_string());
    t
}

/// Build a ReconciliationConfig with freshness-specific defaults suitable for tests.
///
/// Callers can mutate returned config before passing to `ensure_branches_fresh`.
fn freshness_config() -> ReconciliationConfig {
    ReconciliationConfig {
        // Use a short timeout so tests don't hang on failures
        branch_freshness_timeout_secs: 10,
        freshness_skip_window_secs: 30,
        freshness_max_conflict_retries: 3,
        execution_freshness_enabled: true,
        ..Default::default()
    }
}

// ==================
// Config toggle tests
// ==================

#[tokio::test]
async fn config_disabled_returns_ok_without_checking() {
    // When execution_freshness_enabled=false, the function must return Ok immediately
    // without performing any git operations.  We pass a nonexistent path — if git
    // were invoked, it would fail, exposing an early exit that didn't happen.
    let mut cfg = freshness_config();
    cfg.execution_freshness_enabled = false;

    let task = make_test_task(Some("task/branch"), None);
    let project = make_test_project("/nonexistent/path/should-not-be-accessed");
    let nonexistent = std::path::Path::new("/nonexistent/path/should-not-be-accessed");

    let result = ensure_branches_fresh(
        nonexistent,
        &task,
        &project,
        "task-disabled",
        Some("plan/feature-1"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Disabled config must return Ok immediately. Got: {:?}",
        result
    );
}

// ==================
// Skip-if-recently-checked tests
// ==================

#[tokio::test]
async fn skip_window_within_threshold_returns_ok_skipped() {
    // last_freshness_check_at was 5 seconds ago, skip_window=30 → skip and return Ok.
    let last_check = (Utc::now() - chrono::Duration::seconds(5)).to_rfc3339();
    let metadata = serde_json::json!({
        "last_freshness_check_at": last_check,
        "freshness_conflict_count": 0,
    });
    let task = make_test_task(Some("task/branch"), Some(metadata));

    let mut cfg = freshness_config();
    cfg.freshness_skip_window_secs = 30;

    // Use a nonexistent path — no git calls should happen.
    let project = make_test_project("/nonexistent/skip-check-path");
    let nonexistent = std::path::Path::new("/nonexistent/skip-check-path");

    let result = ensure_branches_fresh(
        nonexistent,
        &task,
        &project,
        "task-skip",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Within skip window should return Ok without git calls. Got: {:?}",
        result
    );
    // The returned metadata should still have the original last_freshness_check_at
    let meta = result.unwrap();
    assert!(
        meta.last_freshness_check_at.is_some(),
        "Skipped check should preserve last_freshness_check_at"
    );
}

#[tokio::test]
async fn skip_window_expired_proceeds_with_check() {
    // last_freshness_check_at was 60 seconds ago, skip_window=30 → check runs.
    // Use a real git repo that will pass both checks so we get Ok back.
    let repo = setup_real_git_repo();
    let last_check = (Utc::now() - chrono::Duration::seconds(60)).to_rfc3339();
    let metadata = serde_json::json!({
        "last_freshness_check_at": last_check,
        "freshness_conflict_count": 0,
    });
    let task = make_test_task(Some(&repo.task_branch), Some(metadata));

    let mut cfg = freshness_config();
    cfg.freshness_skip_window_secs = 30;

    let project = make_test_project(&repo.path_string());

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-expired-skip",
        None, // no plan branch → skip plan check
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Expired skip window with fresh repo should return Ok. Got: {:?}",
        result
    );
    // The returned metadata should have a freshened timestamp (i.e. more recent than original)
    let meta = result.unwrap();
    let new_ts = meta
        .last_freshness_check_at
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .expect("last_freshness_check_at should be set after a successful check");
    let old_ts: chrono::DateTime<Utc> = last_check.parse().unwrap();
    assert!(
        new_ts > old_ts,
        "Timestamp should be refreshed after a real check. old={old_ts}, new={new_ts}"
    );
}

// ==================
// Plan check result mapping tests
// ==================

#[tokio::test]
async fn plan_already_up_to_date_continues_to_source() {
    // Plan branch at same commit as main → AlreadyUpToDate → source check also passes → Ok.
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from current main (no divergence)
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/fresh-plan"])
        .current_dir(path)
        .output()
        .expect("git branch");

    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-plan-up-to-date",
        Some("plan/fresh-plan"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Plan up-to-date + source up-to-date should return Ok. Got: {:?}",
        result
    );
    let meta = result.unwrap();
    assert!(
        !meta.branch_freshness_conflict,
        "No conflict should be flagged"
    );
}

#[tokio::test]
async fn plan_updated_continues_to_source() {
    // Plan branch behind main → Updated → source check also passes → Ok.
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from main (before new commit)
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/behind-plan"])
        .current_dir(path)
        .output()
        .expect("git branch plan/behind-plan");

    // Add a commit to main (plan branch is now behind)
    std::fs::write(path.join("main_fix.rs"), "// main fix").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "main_fix.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main fix for plan-behind test"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-plan-updated",
        Some("plan/behind-plan"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Plan updated (behind→fresh) + source up-to-date should return Ok. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn plan_not_plan_branch_skipped() {
    // plan_branch=None → plan check skipped entirely → source check runs → Ok.
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-no-plan",
        None, // no plan branch
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "No plan branch → skip plan check → source passes → Ok. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn plan_conflicts_returns_route_to_merging() {
    // Plan branch and main have diverging changes on same file → conflict → RouteToMerging.
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from main
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/conflicting"])
        .current_dir(path)
        .output();

    // Add a conflicting change to main
    std::fs::write(path.join("shared.rs"), "// main version\nfn main_fn() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "shared.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main changes shared.rs"])
        .current_dir(path)
        .output();

    // Add a conflicting change on the plan branch
    let _ = std::process::Command::new("git")
        .args(["checkout", "plan/conflicting"])
        .current_dir(path)
        .output();
    std::fs::write(path.join("shared.rs"), "// plan version\nfn plan_fn() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "shared.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "feat: plan changes shared.rs"])
        .current_dir(path)
        .output();

    // Back to main
    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-plan-conflict",
        Some("plan/conflicting"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        matches!(
            result,
            Err(FreshnessAction::RouteToMerging { conflict_type: "plan_update", .. })
        ),
        "Plan conflict must return RouteToMerging with conflict_type=plan_update. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn plan_error_is_non_fatal_continues() {
    // If plan_branch doesn't exist, update_plan_from_main returns Error (non-fatal).
    // ensure_branches_fresh should warn and continue to source check.
    // Source check succeeds → Ok.
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-plan-error",
        Some("plan/nonexistent-plan-branch"), // branch doesn't exist → Error (non-fatal)
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Plan check error is non-fatal; source check should succeed → Ok. Got: {:?}",
        result
    );
}

// ==================
// Source check result mapping tests
// ==================

#[tokio::test]
async fn source_already_up_to_date_returns_ok() {
    // Task branch is up-to-date with main, no plan branch → Ok.
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-source-up-to-date",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Source up-to-date → Ok. Got: {:?}",
        result
    );
    let meta = result.unwrap();
    assert!(
        !meta.branch_freshness_conflict,
        "No conflict should be flagged for up-to-date source"
    );
}

#[tokio::test]
async fn source_updated_returns_ok() {
    // Main has a new commit after task branch was created.  Source update merges it in → Ok.
    let repo = setup_real_git_repo();
    let path = repo.path();

    // New commit on main (task branch is now behind)
    std::fs::write(path.join("main_extra.rs"), "// main extra").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "main_extra.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: extra commit on main"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-source-updated",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Source updated (behind→fresh) should return Ok. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn source_conflicts_returns_route_to_merging() {
    // Main and task branch have conflicting changes on the same file → source conflict →
    // RouteToMerging with conflict_type=source_update.
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Add a conflicting change on main (same file modified by task branch: feature.rs)
    std::fs::write(
        path.join("feature.rs"),
        "// main conflicting version\nfn main_feature() {}",
    )
    .unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "feature.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main modifies feature.rs"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());
    let task = make_test_task(Some(&repo.task_branch), None);
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-source-conflict",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        matches!(
            result,
            Err(FreshnessAction::RouteToMerging { conflict_type: "source_update", .. })
        ),
        "Source conflict must return RouteToMerging with conflict_type=source_update. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn source_error_is_non_fatal_returns_ok() {
    // Empty task_branch → source check is skipped entirely → Ok.
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());
    let task = make_test_task(None, None); // no task_branch → source check skipped
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-no-branch",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        result.is_ok(),
        "Empty task_branch skips source check → Ok. Got: {:?}",
        result
    );
}

// ==================
// Retry counting tests
// ==================

#[tokio::test]
async fn plan_conflict_increments_count_and_routes() {
    // conflict_count starts at 0, conflict detected → count becomes 1 → RouteToMerging (not blocked).
    let repo = setup_real_git_repo();
    let path = repo.path();

    setup_plan_conflict(path, "plan/count-test-1");

    let project = make_test_project(&repo.path_string());
    // Start with count=0
    let metadata = serde_json::json!({ "freshness_conflict_count": 0 });
    let task = make_test_task(Some(&repo.task_branch), Some(metadata));
    let mut cfg = freshness_config();
    cfg.freshness_max_conflict_retries = 3;

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-count-0",
        Some("plan/count-test-1"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    match result {
        Err(FreshnessAction::RouteToMerging { freshness_metadata, .. }) => {
            assert_eq!(
                freshness_metadata.freshness_conflict_count, 1,
                "Count should be incremented from 0 to 1"
            );
        }
        other => panic!("Expected RouteToMerging, got: {other:?}"),
    }
}

#[tokio::test]
async fn plan_conflict_at_cap_returns_blocked() {
    // conflict_count starts at 3, cap=3 → count becomes 4 > 3 → ExecutionBlocked.
    let repo = setup_real_git_repo();
    let path = repo.path();

    setup_plan_conflict(path, "plan/count-cap-test");

    let project = make_test_project(&repo.path_string());
    // Start with count already at cap
    let metadata = serde_json::json!({ "freshness_conflict_count": 3 });
    let task = make_test_task(Some(&repo.task_branch), Some(metadata));
    let mut cfg = freshness_config();
    cfg.freshness_max_conflict_retries = 3;

    let result = ensure_branches_fresh(
        path,
        &task,
        &project,
        "task-at-cap",
        Some("plan/count-cap-test"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    assert!(
        matches!(result, Err(FreshnessAction::ExecutionBlocked { .. })),
        "Count at cap (3+1=4 > 3) must return ExecutionBlocked. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn success_resets_conflict_count() {
    // Start with conflict_count=2, both branches fresh → Ok, returned metadata has count=0.
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());
    let metadata = serde_json::json!({ "freshness_conflict_count": 2 });
    let task = make_test_task(Some(&repo.task_branch), Some(metadata));
    let cfg = freshness_config();

    let result = ensure_branches_fresh(
        repo.path(),
        &task,
        &project,
        "task-reset-count",
        None,
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    match result {
        Ok(meta) => {
            assert_eq!(
                meta.freshness_conflict_count, 0,
                "Successful check must reset conflict count to 0"
            );
            assert!(
                !meta.branch_freshness_conflict,
                "branch_freshness_conflict must be cleared on success"
            );
        }
        Err(e) => panic!("Expected Ok with reset count, got: {e:?}"),
    }
}

// ==================
// Dual-conflict scenario
// ==================

/// Tests the full dual-conflict flow across three sequential calls:
///
/// Call 1: plan has conflicts → RouteToMerging (count=1)
/// Call 2: task metadata has count=1, plan now fresh but source has conflicts → RouteToMerging (count=2)
/// Call 3: task metadata has count=2, both fresh → Ok (count reset to 0)
#[tokio::test]
async fn dual_conflict_sequential_plan_then_source() {
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Setup plan conflict: plan/dual-conflict-plan diverges from main
    setup_plan_conflict(path, "plan/dual-conflict-plan");

    let project = make_test_project(&repo.path_string());
    let mut cfg = freshness_config();
    cfg.freshness_max_conflict_retries = 5; // plenty of room

    // --- Call 1: plan has conflict ---
    let task1 = make_test_task(Some(&repo.task_branch), None);
    let result1 = ensure_branches_fresh(
        path,
        &task1,
        &project,
        "task-dual",
        Some("plan/dual-conflict-plan"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    let freshness_after_call1 = match result1 {
        Err(FreshnessAction::RouteToMerging { freshness_metadata, conflict_type: "plan_update", .. }) => {
            assert_eq!(freshness_metadata.freshness_conflict_count, 1, "Call 1 must set count=1");
            freshness_metadata
        }
        other => panic!("Call 1 expected RouteToMerging(plan_update), got: {other:?}"),
    };

    // Simulate: merger agent resolved the plan conflict (plan branch now merged main).
    // We do a real git merge abort + fast-forward so the next plan check passes.
    resolve_plan_conflict(path, "plan/dual-conflict-plan");

    // Now set up a source conflict: main gets a new change that conflicts with task branch
    setup_source_conflict(path, &repo.task_branch);

    // --- Call 2: plan fresh, source has conflict ---
    // Carry forward the metadata from call 1 (count=1)
    let meta_json = serde_json::to_value(&freshness_after_call1).unwrap();
    let task2 = make_test_task(Some(&repo.task_branch), Some(meta_json));

    let result2 = ensure_branches_fresh(
        path,
        &task2,
        &project,
        "task-dual",
        Some("plan/dual-conflict-plan"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    let freshness_after_call2 = match result2 {
        Err(FreshnessAction::RouteToMerging { freshness_metadata, conflict_type: "source_update", .. }) => {
            assert_eq!(freshness_metadata.freshness_conflict_count, 2, "Call 2 must set count=2");
            freshness_metadata
        }
        other => panic!("Call 2 expected RouteToMerging(source_update), got: {other:?}"),
    };

    // Simulate: merger agent resolved the source conflict.
    resolve_source_conflict(path, &repo.task_branch);

    // --- Call 3: both branches fresh ---
    let meta_json3 = serde_json::to_value(&freshness_after_call2).unwrap();
    let task3 = make_test_task(Some(&repo.task_branch), Some(meta_json3));

    let result3 = ensure_branches_fresh(
        path,
        &task3,
        &project,
        "task-dual",
        Some("plan/dual-conflict-plan"),
        None,
        None,
        "executing",
        &cfg,
    )
    .await;

    match result3 {
        Ok(meta) => {
            assert_eq!(
                meta.freshness_conflict_count, 0,
                "Call 3 (both fresh) must reset conflict count to 0"
            );
        }
        Err(e) => panic!("Call 3 expected Ok, got: {e:?}"),
    }
}

// ==================
// Git setup helpers for conflict tests
// ==================

/// Create a plan branch from main, then add conflicting commits on both sides.
/// After this, `plan/plan_branch_name` and `main` have diverging changes on `shared_plan.rs`.
///
/// Leaves repo on `main`.
fn setup_plan_conflict(path: &std::path::Path, plan_branch_name: &str) {
    // Create plan branch from current main HEAD
    let _ = std::process::Command::new("git")
        .args(["branch", plan_branch_name])
        .current_dir(path)
        .output()
        .expect("git branch plan");

    // Add conflicting commit to main
    let filename = format!("shared_{}.rs", plan_branch_name.replace('/', "_"));
    std::fs::write(
        path.join(&filename),
        "// main version\nfn main_impl() {}",
    )
    .unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", &filename])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", &format!("fix: main changes {}", filename)])
        .current_dir(path)
        .output();

    // Add conflicting commit on plan branch
    let _ = std::process::Command::new("git")
        .args(["checkout", plan_branch_name])
        .current_dir(path)
        .output();
    std::fs::write(
        path.join(&filename),
        "// plan version\nfn plan_impl() {}",
    )
    .unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", &filename])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", &format!("feat: plan changes {}", filename)])
        .current_dir(path)
        .output();

    // Back to main
    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();
}

/// Resolve a plan conflict by checking out the plan branch, using `ours` strategy for
/// the conflicting file, committing, then switching back to main.
///
/// This simulates what a merger agent does after resolving a plan_update conflict.
fn resolve_plan_conflict(path: &std::path::Path, plan_branch_name: &str) {
    let _ = std::process::Command::new("git")
        .args(["checkout", plan_branch_name])
        .current_dir(path)
        .output();

    // Merge main into plan using ours strategy (simulate agent resolution)
    let merge_out = std::process::Command::new("git")
        .args(["merge", "main", "-X", "ours", "--no-edit"])
        .current_dir(path)
        .output()
        .expect("git merge");

    if !merge_out.status.success() {
        // If merge failed for some reason, abort and use checkout --ours
        let _ = std::process::Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(path)
            .output();
    }

    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();
}

/// Create a source conflict: add a commit to main that conflicts with what's in task branch.
///
/// The `setup_real_git_repo()` task branch already has `feature.rs`. This adds a conflicting
/// version of `feature.rs` on main so the source update will conflict.
fn setup_source_conflict(path: &std::path::Path, task_branch: &str) {
    // We're on main — add a conflicting change to feature.rs
    std::fs::write(
        path.join("feature.rs"),
        "// main conflicting source version\nfn main_source() {}",
    )
    .unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "feature.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: main changes feature.rs (source conflict)"])
        .current_dir(path)
        .output();

    // Verify the task branch has the old version (it should from setup_real_git_repo)
    let _ = task_branch; // suppress unused warning
}

/// Resolve a source conflict by merging main into the task branch with `ours` strategy.
///
/// Simulates what a merger agent does after resolving a source_update conflict.
fn resolve_source_conflict(path: &std::path::Path, task_branch: &str) {
    let _ = std::process::Command::new("git")
        .args(["checkout", task_branch])
        .current_dir(path)
        .output();

    // Merge main using ours strategy to resolve the conflict
    let merge_out = std::process::Command::new("git")
        .args(["merge", "main", "-X", "ours", "--no-edit"])
        .current_dir(path)
        .output()
        .expect("git merge for source conflict resolution");

    if !merge_out.status.success() {
        // abort any in-progress merge to leave repo in clean state
        let _ = std::process::Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(path)
            .output();
    }

    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();
}
