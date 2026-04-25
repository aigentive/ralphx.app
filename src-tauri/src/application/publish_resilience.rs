use std::path::Path;
use std::sync::Arc;

use crate::domain::state_machine::transition_handler::{
    classify_commit_hook_failure_text, update_source_from_target, CommitHookFailureKind,
    SourceUpdateResult,
};
use crate::domain::services::GithubServiceTrait;
use crate::error::AppResult;
use crate::{application::GitService, domain::entities::Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishFailureClass {
    AgentFixable,
    Operational,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishBranchFreshnessOutcome {
    AlreadyFresh {
        base_commit: String,
        target_ref: String,
    },
    Updated {
        base_commit: String,
        target_ref: String,
    },
    NeedsAgent {
        message: String,
        conflict_files: Vec<String>,
        base_commit: String,
        target_ref: String,
    },
    OperationalError {
        message: String,
    },
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

pub async fn count_publish_reviewable_commits(
    repo_path: &Path,
    source_branch: &str,
    review_base: &str,
) -> AppResult<u32> {
    GitService::count_commits_not_on_branch(repo_path, source_branch, review_base).await
}

pub async fn count_existing_publish_branch_reviewable_commits(
    repo_path: &Path,
    source_branch: &str,
    review_base: &str,
) -> AppResult<u32> {
    if !GitService::branch_exists(repo_path, source_branch)
        .await
        .unwrap_or(false)
    {
        return Ok(0);
    }

    count_publish_reviewable_commits(repo_path, source_branch, review_base).await
}

pub async fn push_publish_branch(
    github: &Arc<dyn GithubServiceTrait>,
    repo_path: &Path,
    branch: &str,
) -> AppResult<()> {
    github.push_branch(repo_path, branch).await
}

pub async fn ensure_publish_branch_fresh(
    repo_path: &Path,
    project: &Project,
    source_branch: &str,
    base_ref: &str,
    conversation_id: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> PublishBranchFreshnessOutcome {
    if let Err(error) = GitService::fetch_origin(repo_path).await {
        return PublishBranchFreshnessOutcome::OperationalError {
            message: format!("Failed to refresh git remotes before publishing: {error}"),
        };
    }

    let target_ref = resolve_publish_freshness_target(repo_path, base_ref).await;
    let target_sha = match GitService::get_branch_sha(repo_path, &target_ref).await {
        Ok(sha) => sha,
        Err(error) => {
            return PublishBranchFreshnessOutcome::OperationalError {
                message: format!(
                    "Failed to resolve publish base ref '{}' before publishing: {}",
                    target_ref, error
                ),
            };
        }
    };

    let result = update_source_from_target(
        repo_path,
        source_branch,
        &target_ref,
        project,
        conversation_id,
        app_handle,
    )
    .await;

    publish_branch_freshness_outcome_from_source_update(result, &target_ref, &target_sha)
}

pub(crate) fn publish_branch_freshness_outcome_from_source_update(
    result: SourceUpdateResult,
    target_ref: &str,
    target_sha: &str,
) -> PublishBranchFreshnessOutcome {
    match result {
        SourceUpdateResult::AlreadyUpToDate => PublishBranchFreshnessOutcome::AlreadyFresh {
            base_commit: target_sha.to_string(),
            target_ref: target_ref.to_string(),
        },
        SourceUpdateResult::Updated => PublishBranchFreshnessOutcome::Updated {
            base_commit: target_sha.to_string(),
            target_ref: target_ref.to_string(),
        },
        SourceUpdateResult::Conflicts { conflict_files } => {
            let conflict_files = conflict_files
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            let files_label = if conflict_files.is_empty() {
                "unknown files".to_string()
            } else {
                conflict_files.join(", ")
            };
            PublishBranchFreshnessOutcome::NeedsAgent {
                message: format!(
                    "Merge conflict updating agent workspace branch from {target_ref}: {files_label}"
                ),
                conflict_files,
                base_commit: target_sha.to_string(),
                target_ref: target_ref.to_string(),
            }
        }
        SourceUpdateResult::Error(message) => {
            PublishBranchFreshnessOutcome::OperationalError { message }
        }
    }
}

pub fn remote_tracking_ref_for_publish(base_ref: &str) -> String {
    if base_ref.starts_with("origin/") {
        base_ref.to_string()
    } else {
        format!("origin/{base_ref}")
    }
}

async fn resolve_publish_freshness_target(repo_path: &Path, base_ref: &str) -> String {
    let remote_ref = remote_tracking_ref_for_publish(base_ref);
    if remote_ref != base_ref
        && GitService::ref_exists(repo_path, &remote_ref)
            .await
            .unwrap_or(false)
    {
        remote_ref
    } else {
        base_ref.to_string()
    }
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
        "non-fast-forward",
        "failed to push some refs",
        "updates were rejected",
        "fetch first",
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
