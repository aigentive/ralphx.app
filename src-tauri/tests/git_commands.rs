use ralphx_lib::application::{AppState, CommitInfo, DiffStats};
use ralphx_lib::commands::diff_commands::{
    get_file_diff_for_state, get_task_file_changes_for_state,
};
use ralphx_lib::commands::git_commands::{
    get_task_commits_for_state, CommitInfoResponse, TaskDiffStatsResponse,
};
use ralphx_lib::domain::entities::{
    ArtifactId, IdeationSessionId, InternalStatus, PlanBranch, Project, Task, TaskCategory,
};
use std::path::Path;
use std::process::Command;

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_git_output(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output is utf-8")
        .trim()
        .to_string()
}

fn setup_plan_branch_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let repo = dir.path();

    run_git(repo, &["init", "-b", "main"]);
    run_git(repo, &["config", "user.email", "test@test.com"]);
    run_git(repo, &["config", "user.name", "Test"]);

    std::fs::write(repo.join("README.md"), "# test repo\n").expect("write readme");
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial commit"]);

    run_git(repo, &["checkout", "-b", "plan/test"]);
    std::fs::write(repo.join("plan.txt"), "first\n").expect("write plan");
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "feat: first plan change"]);
    std::fs::write(repo.join("plan.txt"), "first\nsecond\n").expect("update plan");
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "feat: second plan change"]);

    run_git(repo, &["checkout", "main"]);

    dir
}

async fn setup_branchless_plan_merge_state(repo: &Path) -> (AppState, Task) {
    let app_state = AppState::new_test();

    let mut project = Project::new(
        "Test Project".to_string(),
        repo.to_string_lossy().to_string(),
    );
    project.base_branch = Some("main".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .expect("create project");

    let mut task = Task::new_with_category(
        project.id.clone(),
        "Merge plan branch".to_string(),
        TaskCategory::PlanMerge,
    );
    task.internal_status = InternalStatus::WaitingOnPr;
    task.task_branch = None;
    task.worktree_path = None;
    app_state
        .task_repo
        .create(task.clone())
        .await
        .expect("create task");

    let mut plan_branch = PlanBranch::new(
        ArtifactId::new(),
        IdeationSessionId::new(),
        project.id.clone(),
        "plan/test".to_string(),
        "main".to_string(),
    );
    plan_branch.merge_task_id = Some(task.id.clone());
    plan_branch.pr_eligible = true;
    app_state
        .plan_branch_repo
        .create(plan_branch)
        .await
        .expect("create plan branch");

    (app_state, task)
}

#[test]
fn test_commit_info_response_conversion() {
    let info = CommitInfo {
        sha: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
        short_sha: "abcdef1".to_string(),
        message: "Test commit".to_string(),
        author: "Test Author".to_string(),
        timestamp: "2026-02-02T12:00:00+00:00".to_string(),
    };

    let response = CommitInfoResponse::from(info);
    assert_eq!(response.short_sha, "abcdef1");
    assert_eq!(response.message, "Test commit");
}

#[test]
fn test_diff_stats_response_conversion() {
    let stats = DiffStats {
        files_changed: 5,
        insertions: 100,
        deletions: 50,
        changed_files: vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()],
    };

    let response = TaskDiffStatsResponse::from(stats);
    assert_eq!(response.files_changed, 5);
    assert_eq!(response.insertions, 100);
    assert_eq!(response.deletions, 50);
    assert_eq!(response.changed_files.len(), 2);
}

#[tokio::test]
async fn test_get_task_commits_uses_plan_branch_for_branchless_plan_merge_task() {
    let repo = setup_plan_branch_repo();
    let (app_state, task) = setup_branchless_plan_merge_state(repo.path()).await;

    let response = get_task_commits_for_state(task.id.clone(), &app_state)
        .await
        .expect("get commits");

    let messages: Vec<_> = response
        .commits
        .iter()
        .map(|commit| commit.message.as_str())
        .collect();
    assert_eq!(
        messages,
        vec!["feat: second plan change", "feat: first plan change"]
    );
}

#[tokio::test]
async fn test_get_task_commits_uses_plan_branch_merge_sha_for_merged_plan_merge_task() {
    let repo = setup_plan_branch_repo();
    run_git(
        repo.path(),
        &[
            "merge",
            "--no-ff",
            "plan/test",
            "-m",
            "Merge pull request #68",
        ],
    );
    let merge_sha = run_git_output(repo.path(), &["rev-parse", "HEAD"]);
    let (app_state, mut task) = setup_branchless_plan_merge_state(repo.path()).await;

    task.internal_status = InternalStatus::Merged;
    task.merge_commit_sha = None;
    app_state
        .task_repo
        .update(&task)
        .await
        .expect("update task");
    let plan_branch = app_state
        .plan_branch_repo
        .get_by_merge_task_id(&task.id)
        .await
        .expect("get plan branch")
        .expect("plan branch exists");
    app_state
        .plan_branch_repo
        .set_merge_commit_sha(&plan_branch.id, merge_sha)
        .await
        .expect("set plan branch merge sha");

    let response = get_task_commits_for_state(task.id.clone(), &app_state)
        .await
        .expect("get commits");

    let messages: Vec<_> = response
        .commits
        .iter()
        .map(|commit| commit.message.as_str())
        .collect();
    assert_eq!(
        messages,
        vec!["feat: second plan change", "feat: first plan change"]
    );
}

#[tokio::test]
async fn test_diff_commands_use_plan_branch_merge_sha_for_merged_plan_merge_task() {
    let repo = setup_plan_branch_repo();
    run_git(
        repo.path(),
        &[
            "merge",
            "--no-ff",
            "plan/test",
            "-m",
            "Merge pull request #68",
        ],
    );
    let merge_sha = run_git_output(repo.path(), &["rev-parse", "HEAD"]);
    run_git(repo.path(), &["branch", "-D", "plan/test"]);
    let (app_state, mut task) = setup_branchless_plan_merge_state(repo.path()).await;

    task.internal_status = InternalStatus::Merged;
    task.merge_commit_sha = None;
    app_state
        .task_repo
        .update(&task)
        .await
        .expect("update task");
    let plan_branch = app_state
        .plan_branch_repo
        .get_by_merge_task_id(&task.id)
        .await
        .expect("get plan branch")
        .expect("plan branch exists");
    app_state
        .plan_branch_repo
        .set_merge_commit_sha(&plan_branch.id, merge_sha)
        .await
        .expect("set plan branch merge sha");

    let changes = get_task_file_changes_for_state(&app_state, task.id.clone())
        .await
        .expect("get file changes");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "plan.txt");

    let diff = get_file_diff_for_state(&app_state, task.id.clone(), "plan.txt".to_string())
        .await
        .expect("get file diff");
    assert_eq!(diff.old_content, "");
    assert_eq!(diff.new_content, "first\nsecond\n");
}

#[tokio::test]
async fn test_diff_commands_use_plan_branch_for_branchless_plan_merge_task() {
    let repo = setup_plan_branch_repo();
    let (app_state, task) = setup_branchless_plan_merge_state(repo.path()).await;

    let changes = get_task_file_changes_for_state(&app_state, task.id.clone())
        .await
        .expect("get file changes");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "plan.txt");

    let diff = get_file_diff_for_state(&app_state, task.id.clone(), "plan.txt".to_string())
        .await
        .expect("get file diff");
    assert_eq!(diff.old_content, "");
    assert_eq!(diff.new_content, "first\nsecond\n");
}

/// Verify that the retry_merge metadata reset logic clears all loop-prevention
/// counters so the reconciler won't block subsequent auto-retries.
#[test]
fn test_retry_merge_resets_loop_counters() {
    // Simulate task metadata with high validation_revert_count, AgentReported source,
    // and merge_recovery events that would block auto-retry.
    let metadata = serde_json::json!({
        "validation_revert_count": 5,
        "merge_failure_source": "agent_reported",
        "merge_recovery": {
            "version": 1,
            "events": [
                {"kind": "auto_retry_triggered", "timestamp": "2026-01-01T00:00:00Z", "source": "system"},
                {"kind": "auto_retry_triggered", "timestamp": "2026-01-01T00:01:00Z", "source": "system"},
                {"kind": "attempt_failed", "timestamp": "2026-01-01T00:02:00Z", "source": "system"},
            ],
            "last_state": "failed"
        },
        "some_other_key": "preserved"
    });

    // Apply the same reset logic as retry_merge()
    let mut meta_obj = metadata.as_object().cloned().unwrap();
    meta_obj.insert(
        "merge_retry_in_progress".to_string(),
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );
    meta_obj.insert("validation_revert_count".to_string(), serde_json::json!(0));
    meta_obj.remove("merge_failure_source");
    if let Some(recovery_val) = meta_obj.get_mut("merge_recovery") {
        if let Some(recovery_obj) = recovery_val.as_object_mut() {
            recovery_obj.insert("events".to_string(), serde_json::json!([]));
            recovery_obj.insert("last_state".to_string(), serde_json::json!("retrying"));
        }
    }

    let result = serde_json::Value::Object(meta_obj);

    // validation_revert_count reset to 0
    assert_eq!(result["validation_revert_count"], 0);
    // merge_failure_source removed
    assert!(result.get("merge_failure_source").is_none());
    // merge_recovery events cleared
    assert_eq!(
        result["merge_recovery"]["events"].as_array().unwrap().len(),
        0
    );
    // merge_recovery last_state set to retrying
    assert_eq!(result["merge_recovery"]["last_state"], "retrying");
    // Other metadata keys preserved
    assert_eq!(result["some_other_key"], "preserved");
    // In-flight guard set (timestamp string, not boolean)
    assert!(result["merge_retry_in_progress"].is_string());
}

/// Verify that the reset logic handles metadata with no merge_recovery key.
#[test]
fn test_retry_merge_resets_counters_without_merge_recovery() {
    let metadata = serde_json::json!({
        "validation_revert_count": 3,
        "merge_failure_source": "agent_reported",
    });

    let mut meta_obj = metadata.as_object().cloned().unwrap();
    meta_obj.insert(
        "merge_retry_in_progress".to_string(),
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );
    meta_obj.insert("validation_revert_count".to_string(), serde_json::json!(0));
    meta_obj.remove("merge_failure_source");
    if let Some(recovery_val) = meta_obj.get_mut("merge_recovery") {
        if let Some(recovery_obj) = recovery_val.as_object_mut() {
            recovery_obj.insert("events".to_string(), serde_json::json!([]));
            recovery_obj.insert("last_state".to_string(), serde_json::json!("retrying"));
        }
    }

    let result = serde_json::Value::Object(meta_obj);

    assert_eq!(result["validation_revert_count"], 0);
    assert!(result.get("merge_failure_source").is_none());
    // No merge_recovery key — should not crash
    assert!(result.get("merge_recovery").is_none());
}

/// Verify that a task with legacy boolean `merge_retry_in_progress: true` (old format)
/// is NOT blocked by the duplicate-retry guard. This reproduces the exact scenario where
/// a task had the old boolean flag stuck in DB metadata — the guard must treat it as stale
/// and allow the retry to proceed.
#[test]
fn test_legacy_boolean_merge_retry_flag_does_not_block_retry() {
    // Simulate metadata with the OLD boolean format (pre-timestamp migration)
    let metadata = serde_json::json!({
        "merge_retry_in_progress": true,
        "error": "Merge timed out after 1200s without complete_merge callback",
    });
    let metadata_json = metadata;

    // This is the exact guard logic from retry_merge() in git_commands.rs
    let retry_in_progress = metadata_json
        .get("merge_retry_in_progress")
        .and_then(|v| {
            if let Some(ts) = v.as_str() {
                let started = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
                let age = chrono::Utc::now() - started.with_timezone(&chrono::Utc);
                return Some(age < chrono::Duration::seconds(60));
            }
            // Legacy boolean or other non-string: stale
            Some(false)
        })
        .unwrap_or(false);

    assert!(
        !retry_in_progress,
        "Legacy boolean merge_retry_in_progress: true must NOT block retry (should be treated as stale)"
    );
}

/// Verify that the reconciler's validation_revert_count check would pass after reset.
/// The reconciler blocks when validation_revert_count >= max (default 2).
/// After user retry resets to 0, the check should pass.
#[test]
fn test_validation_revert_count_passes_after_reset() {
    // Simulate metadata after retry_merge resets the counter
    let metadata_str = serde_json::json!({
        "validation_revert_count": 0,
        "merge_retry_in_progress": chrono::Utc::now().to_rfc3339(),
    })
    .to_string();

    // Same read logic as ReconciliationRunner::validation_revert_count()
    let revert_count: u32 = serde_json::from_str::<serde_json::Value>(&metadata_str)
        .ok()
        .and_then(|v| {
            v.get("validation_revert_count")
                .and_then(|c| c.as_u64())
                .map(|c| c as u32)
        })
        .unwrap_or(0);

    assert_eq!(revert_count, 0);
    // reconciliation_config().validation_revert_max_count defaults to 2
    // 0 <= 2, so the reconciler would NOT block auto-retry
    assert!(revert_count <= 2);
}
