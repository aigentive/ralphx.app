use super::publish_resilience::review_base_for_publish;
use super::publish_resilience::{
    classify_publish_failure, publish_branch_freshness_outcome_from_source_update,
    publish_branch_freshness_status_from_commits,
    publish_branch_freshness_status_from_commits_and_branch, remote_tracking_ref_for_publish,
    verify_agent_workspace_repair_completion, AgentWorkspaceRepairCompletionCheck,
    PublishBranchFreshnessOutcome, PublishFailureClass,
};
use crate::domain::state_machine::transition_handler::SourceUpdateResult;
use std::path::PathBuf;

#[test]
fn classifies_commit_hook_policy_failures_as_agent_fixable() {
    let error = "Failed to commit changes: pre-commit hook failed: npm run typecheck failed";

    assert_eq!(
        classify_publish_failure(error),
        PublishFailureClass::AgentFixable
    );
}

#[test]
fn classifies_branch_conflicts_as_agent_fixable() {
    let error = "failed to update branch: merge conflict in frontend/src/App.tsx";

    assert_eq!(
        classify_publish_failure(error),
        PublishFailureClass::AgentFixable
    );
}

#[test]
fn classifies_non_fast_forward_push_rejections_as_agent_fixable() {
    let error = "failed to push some refs: updates were rejected because the tip of your current branch is behind its remote counterpart (non-fast-forward)";

    assert_eq!(
        classify_publish_failure(error),
        PublishFailureClass::AgentFixable
    );
}

#[test]
fn classifies_github_availability_as_operational() {
    let error = "GitHub integration is not available";

    assert_eq!(
        classify_publish_failure(error),
        PublishFailureClass::Operational
    );
}

#[test]
fn classifies_commit_hook_environment_failures_as_operational() {
    let error = "Failed to commit changes: pre-commit failed: Cannot find package 'vitest'";

    assert_eq!(
        classify_publish_failure(error),
        PublishFailureClass::Operational
    );
}

#[test]
fn requires_captured_base_commit_for_publish_review_base() {
    assert_eq!(
        review_base_for_publish(Some("abc123"), "main").expect("captured commit"),
        "abc123"
    );

    let error = review_base_for_publish(None, "main").expect_err("missing base commit");
    assert!(error.contains("captured base commit"));
}

#[test]
fn maps_source_update_conflicts_to_agent_fixable_publish_outcome() {
    let outcome = publish_branch_freshness_outcome_from_source_update(
        SourceUpdateResult::Conflicts {
            conflict_files: vec![PathBuf::from("frontend/src/App.tsx")],
        },
        "origin/main",
        "target-sha",
    );

    let PublishBranchFreshnessOutcome::NeedsAgent {
        message,
        conflict_files,
        base_commit,
        target_ref,
    } = outcome
    else {
        panic!("expected conflict to route to agent");
    };

    assert_eq!(conflict_files, vec!["frontend/src/App.tsx"]);
    assert_eq!(base_commit, "target-sha");
    assert_eq!(target_ref, "origin/main");
    assert_eq!(
        classify_publish_failure(&message),
        PublishFailureClass::AgentFixable
    );
}

#[test]
fn maps_successful_source_update_to_updated_publish_base() {
    let outcome = publish_branch_freshness_outcome_from_source_update(
        SourceUpdateResult::Updated,
        "origin/main",
        "target-sha",
    );

    assert_eq!(
        outcome,
        PublishBranchFreshnessOutcome::Updated {
            base_commit: "target-sha".to_string(),
            target_ref: "origin/main".to_string(),
        }
    );
}

#[test]
fn derives_remote_tracking_ref_for_publish_base() {
    assert_eq!(remote_tracking_ref_for_publish("main"), "origin/main");
    assert_eq!(
        remote_tracking_ref_for_publish("origin/main"),
        "origin/main"
    );
}

#[test]
fn reports_publish_base_as_current_when_captured_commit_matches_target() {
    let status =
        publish_branch_freshness_status_from_commits(Some("base-sha"), "origin/main", "base-sha");

    assert_eq!(status.target_ref, "origin/main");
    assert_eq!(status.captured_base_commit.as_deref(), Some("base-sha"));
    assert_eq!(status.target_base_commit, "base-sha");
    assert!(!status.is_base_ahead);
}

#[test]
fn reports_publish_base_as_ahead_when_target_commit_changed() {
    let status =
        publish_branch_freshness_status_from_commits(Some("old-base"), "origin/main", "new-base");

    assert_eq!(status.captured_base_commit.as_deref(), Some("old-base"));
    assert_eq!(status.target_base_commit, "new-base");
    assert!(status.is_base_ahead);
}

#[test]
fn reports_publish_base_as_current_when_source_branch_contains_target_commit() {
    let status = publish_branch_freshness_status_from_commits_and_branch(
        Some("old-base"),
        "origin/main",
        "new-base",
        true,
    );

    assert_eq!(status.captured_base_commit.as_deref(), Some("new-base"));
    assert_eq!(status.target_base_commit, "new-base");
    assert!(!status.is_base_ahead);
}

#[test]
fn keeps_publish_base_ahead_when_source_branch_does_not_contain_target_commit() {
    let status = publish_branch_freshness_status_from_commits_and_branch(
        Some("old-base"),
        "origin/main",
        "new-base",
        false,
    );

    assert_eq!(status.captured_base_commit.as_deref(), Some("old-base"));
    assert_eq!(status.target_base_commit, "new-base");
    assert!(status.is_base_ahead);
}

fn repaired_workspace_check() -> AgentWorkspaceRepairCompletionCheck<'static> {
    let status = Box::leak(Box::new(
        publish_branch_freshness_status_from_commits_and_branch(
            Some("old-base"),
            "origin/main",
            "new-base",
            true,
        ),
    ));

    AgentWorkspaceRepairCompletionCheck {
        freshness_status: status,
        workspace_base_ref: "main",
        resolved_base_ref: "origin/main",
        resolved_base_commit: "new-base",
        repair_commit_sha: "repair-head",
        workspace_head_sha: "repair-head",
        has_uncommitted_changes: false,
        is_merge_in_progress: false,
        is_rebase_in_progress: false,
        has_conflict_markers: false,
    }
}

#[test]
fn verifies_clean_agent_workspace_repair_completion() {
    assert!(verify_agent_workspace_repair_completion(repaired_workspace_check()).is_ok());
}

#[test]
fn rejects_agent_workspace_repair_when_base_still_ahead() {
    let stale_status =
        publish_branch_freshness_status_from_commits(Some("old-base"), "origin/main", "new-base");
    let mut check = repaired_workspace_check();
    check.freshness_status = &stale_status;

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("stale base must reject repair completion");
    assert!(error.contains("still behind"));
}

#[test]
fn rejects_agent_workspace_repair_when_reported_base_commit_mismatches_current_target() {
    let mut check = repaired_workspace_check();
    check.resolved_base_commit = "other-base";

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("mismatched base commit must reject repair completion");
    assert!(error.contains("resolved_base_commit"));
}

#[test]
fn rejects_agent_workspace_repair_when_head_does_not_match_reported_repair_commit() {
    let mut check = repaired_workspace_check();
    check.workspace_head_sha = "different-head";

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("reported repair commit must be current HEAD");
    assert!(error.contains("repair_commit_sha"));
}

#[test]
fn rejects_agent_workspace_repair_when_worktree_is_dirty() {
    let mut check = repaired_workspace_check();
    check.has_uncommitted_changes = true;

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("dirty worktree must reject repair completion");
    assert!(error.contains("uncommitted"));
}

#[test]
fn rejects_agent_workspace_repair_when_merge_is_still_in_progress() {
    let mut check = repaired_workspace_check();
    check.is_merge_in_progress = true;

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("in-progress merge must reject repair completion");
    assert!(error.contains("merge is still in progress"));
}

#[test]
fn rejects_agent_workspace_repair_when_rebase_is_still_in_progress() {
    let mut check = repaired_workspace_check();
    check.is_rebase_in_progress = true;

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("in-progress rebase must reject repair completion");
    assert!(error.contains("rebase is still in progress"));
}

#[test]
fn rejects_agent_workspace_repair_when_conflict_markers_remain() {
    let mut check = repaired_workspace_check();
    check.has_conflict_markers = true;

    let error = verify_agent_workspace_repair_completion(check)
        .expect_err("conflict markers must reject repair completion");
    assert!(error.contains("conflict markers"));
}
