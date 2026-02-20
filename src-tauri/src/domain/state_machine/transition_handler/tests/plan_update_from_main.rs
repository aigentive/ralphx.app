// Tests for update_plan_from_main — ensures plan branches are brought up-to-date
// from main before task→plan merges to prevent false validation failures.

use super::helpers::*;
use super::super::merge_coordination::{update_plan_from_main, PlanUpdateResult};
use crate::domain::entities::Project;

/// Helper: create a real git repo with main, a plan branch (from main), and then
/// add a commit to main (simulating fixes committed after plan branch was created).
fn setup_plan_behind_main() -> (RealGitRepo, String) {
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from main (before the new fix)
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/feature-1"])
        .current_dir(path)
        .output()
        .expect("git branch plan/feature-1");

    // Add a fix commit to main (plan branch is now behind)
    std::fs::write(path.join("fix.rs"), "// clippy fix").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "fix.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "fix: clippy errors"])
        .current_dir(path)
        .output();

    (repo, "plan/feature-1".to_string())
}

/// Helper: create a project entity pointing at a repo path.
fn make_test_project(repo_path: &str) -> Project {
    let mut project = Project::new("test-project".to_string(), repo_path.to_string());
    project.base_branch = Some("main".to_string());
    project
}

#[tokio::test]
async fn update_plan_skips_when_target_is_main() {
    let repo = setup_real_git_repo();
    let project = make_test_project(&repo.path_string());

    let result = update_plan_from_main(
        repo.path(),
        "main",       // target = main
        "main",       // base = main
        &project,
        "task-1",
        None,
    )
    .await;

    assert!(
        matches!(result, PlanUpdateResult::NotPlanBranch),
        "Should skip when target == base. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn update_plan_already_up_to_date() {
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from current main (no divergence)
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/feature-up-to-date"])
        .current_dir(path)
        .output()
        .expect("git branch");

    let project = make_test_project(&repo.path_string());

    let result = update_plan_from_main(
        path,
        "plan/feature-up-to-date",
        "main",
        &project,
        "task-2",
        None,
    )
    .await;

    assert!(
        matches!(result, PlanUpdateResult::AlreadyUpToDate),
        "Plan branch at same point as main should be AlreadyUpToDate. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn update_plan_behind_main_gets_updated() {
    let (repo, plan_branch) = setup_plan_behind_main();
    let project = make_test_project(&repo.path_string());

    let result = update_plan_from_main(
        repo.path(),
        &plan_branch,
        "main",
        &project,
        "task-3",
        None,
    )
    .await;

    assert!(
        matches!(result, PlanUpdateResult::Updated),
        "Plan branch behind main should be Updated. Got: {:?}",
        result
    );

    // Verify: main's fix commit should now be on the plan branch
    let log_output = std::process::Command::new("git")
        .args(["log", "--oneline", &plan_branch])
        .current_dir(repo.path())
        .output()
        .expect("git log");
    let log_str = String::from_utf8_lossy(&log_output.stdout);
    assert!(
        log_str.contains("clippy"),
        "Plan branch should now contain the clippy fix commit from main. Log:\n{}",
        log_str,
    );
}

#[tokio::test]
async fn update_plan_with_conflicts_returns_conflicts() {
    let repo = setup_real_git_repo();
    let path = repo.path();

    // Create plan branch from main
    let _ = std::process::Command::new("git")
        .args(["branch", "plan/conflict-test"])
        .current_dir(path)
        .output();

    // Add a file change on main
    std::fs::write(path.join("shared.rs"), "// main version\nfn main() {}").unwrap();
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
        .args(["checkout", "plan/conflict-test"])
        .current_dir(path)
        .output();
    std::fs::write(path.join("shared.rs"), "// plan version\nfn plan() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "shared.rs"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "feat: plan changes shared.rs"])
        .current_dir(path)
        .output();

    // Switch back to main
    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());

    let result = update_plan_from_main(
        path,
        "plan/conflict-test",
        "main",
        &project,
        "task-4",
        None,
    )
    .await;

    assert!(
        matches!(result, PlanUpdateResult::Conflicts { .. }),
        "Conflicting changes should return Conflicts. Got: {:?}",
        result
    );
}

#[tokio::test]
async fn update_plan_nonexistent_base_branch_returns_error() {
    let repo = setup_real_git_repo();
    let path = repo.path();

    let _ = std::process::Command::new("git")
        .args(["branch", "plan/orphan-test"])
        .current_dir(path)
        .output();

    let project = make_test_project(&repo.path_string());

    let result = update_plan_from_main(
        path,
        "plan/orphan-test",
        "nonexistent-base",  // base branch doesn't exist
        &project,
        "task-5",
        None,
    )
    .await;

    assert!(
        matches!(result, PlanUpdateResult::Error(_)),
        "Nonexistent base branch should return Error. Got: {:?}",
        result
    );
}
