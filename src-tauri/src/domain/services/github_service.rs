// GitHub service abstraction (AD2)
//
// Trait-based design allows production `gh` CLI implementation and mock for tests.
// All methods take `working_dir: &Path` for stateless, multi-project support (AD7).

use async_trait::async_trait;
use std::path::Path;

use crate::AppResult;

/// Status of a GitHub pull request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrStatus {
    Open,
    Closed,
    Merged {
        /// SHA of the merge commit, if available
        merge_commit_sha: Option<String>,
    },
}

/// Abstraction over GitHub operations (production: `gh` CLI, tests: mock)
#[async_trait]
pub trait GithubServiceTrait: Send + Sync {
    /// Create a draft pull request. Returns (pr_number, pr_url).
    async fn create_draft_pr(
        &self,
        working_dir: &Path,
        base: &str,
        head: &str,
        title: &str,
        body_file: &Path,
    ) -> AppResult<(i64, String)>;

    /// Convert an existing draft PR to ready-for-review.
    async fn mark_pr_ready(&self, working_dir: &Path, pr_number: i64) -> AppResult<()>;

    /// Update an existing pull request title/body.
    async fn update_pr_details(
        &self,
        working_dir: &Path,
        pr_number: i64,
        title: &str,
        body_file: &Path,
    ) -> AppResult<()>;

    /// Check the current status of a PR.
    async fn check_pr_status(&self, working_dir: &Path, pr_number: i64) -> AppResult<PrStatus>;

    /// Push a branch to origin.
    async fn push_branch(&self, working_dir: &Path, branch: &str) -> AppResult<()>;

    /// Close (without merging) a pull request.
    async fn close_pr(&self, working_dir: &Path, pr_number: i64) -> AppResult<()>;

    /// Delete a remote branch. Already-deleted branches are treated as no-op.
    async fn delete_remote_branch(&self, working_dir: &Path, branch: &str) -> AppResult<()>;

    /// Fetch a branch from origin.
    async fn fetch_remote(&self, working_dir: &Path, branch: &str) -> AppResult<()>;

    /// Find an existing open PR by head branch. Returns (pr_number, pr_url) if found.
    async fn find_pr_by_head_branch(
        &self,
        working_dir: &Path,
        head: &str,
    ) -> AppResult<Option<(i64, String)>>;
}
