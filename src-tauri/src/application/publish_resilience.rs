use crate::domain::state_machine::transition_handler::{
    classify_commit_hook_failure_text, CommitHookFailureKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishFailureClass {
    AgentFixable,
    Operational,
}

pub fn classify_publish_failure(error: &str) -> PublishFailureClass {
    let normalized = error.to_lowercase();

    if is_operational_failure(&normalized) {
        return PublishFailureClass::Operational;
    }

    match classify_commit_hook_failure_text(error) {
        CommitHookFailureKind::PolicyFailure => return PublishFailureClass::AgentFixable,
        CommitHookFailureKind::EnvironmentFailure => return PublishFailureClass::Operational,
        CommitHookFailureKind::Unknown => {}
    }

    if is_agent_fixable_failure(&normalized) {
        return PublishFailureClass::AgentFixable;
    }

    PublishFailureClass::Operational
}

pub fn publish_push_status_for_failure(error: &str) -> &'static str {
    match classify_publish_failure(error) {
        PublishFailureClass::AgentFixable => "needs_agent",
        PublishFailureClass::Operational => "failed",
    }
}

pub fn review_base_for_publish<'a>(
    captured_base_commit: Option<&'a str>,
    base_ref: &str,
) -> Result<&'a str, String> {
    captured_base_commit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "Agent conversation workspace is missing its captured base commit for base ref '{}'",
                base_ref
            )
        })
}

fn is_agent_fixable_failure(normalized: &str) -> bool {
    const PATTERNS: &[&str] = &[
        "conflict",
        "unmerged paths",
        "<<<<<<<",
        "pre-commit",
        "precommit",
        "typecheck",
        "tsc",
        "clippy",
        "lint",
        "test failed",
        "tests failed",
    ];

    PATTERNS.iter().any(|pattern| normalized.contains(pattern))
}

fn is_operational_failure(normalized: &str) -> bool {
    const PATTERNS: &[&str] = &[
        "github integration is not available",
        "workspace not found",
        "conversation not found",
        "project not found",
        "authentication",
        "authorization",
        "permission denied",
        "cannot find package",
        "could not resolve",
    ];

    PATTERNS.iter().any(|pattern| normalized.contains(pattern))
}
