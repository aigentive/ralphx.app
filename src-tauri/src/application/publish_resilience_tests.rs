use super::publish_resilience::review_base_for_publish;
use super::publish_resilience::{classify_publish_failure, PublishFailureClass};

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
