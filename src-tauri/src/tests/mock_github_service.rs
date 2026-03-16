// MockGithubService — test double for GithubServiceTrait
//
// Configurable per-method return values and call tracking.
// No real `gh` or `git` invocations.

use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::domain::services::github_service::{GithubServiceTrait, PrStatus};
use crate::error::AppError;
use crate::AppResult;

/// Shared state allowing callers to configure responses and inspect call counts.
#[derive(Debug, Default)]
pub struct MockGithubState {
    // --- Configurable responses ---
    pub create_draft_pr_result: Option<AppResult<(i64, String)>>,
    pub mark_pr_ready_result: Option<AppResult<()>>,
    pub check_pr_status_result: Option<AppResult<PrStatus>>,
    pub push_branch_result: Option<AppResult<()>>,
    pub close_pr_result: Option<AppResult<()>>,
    pub delete_remote_branch_result: Option<AppResult<()>>,
    pub fetch_remote_result: Option<AppResult<()>>,
    pub find_pr_by_head_branch_result: Option<AppResult<Option<(i64, String)>>>,

    // --- Call tracking ---
    pub create_draft_pr_calls: u32,
    pub mark_pr_ready_calls: u32,
    pub check_pr_status_calls: u32,
    pub push_branch_calls: u32,
    pub close_pr_calls: u32,
    pub delete_remote_branch_calls: u32,
    pub fetch_remote_calls: u32,
    pub find_pr_by_head_branch_calls: u32,

    // --- Last arguments recorded ---
    pub last_create_draft_pr_args: Option<(String, String, String, String)>,
    pub last_mark_pr_ready_number: Option<i64>,
    pub last_check_pr_status_number: Option<i64>,
    pub last_push_branch_name: Option<String>,
    pub last_close_pr_number: Option<i64>,
    pub last_delete_remote_branch_name: Option<String>,
    /// All branches passed to delete_remote_branch (accumulated across all calls).
    pub all_deleted_remote_branch_names: Vec<String>,
    pub last_fetch_remote_branch_name: Option<String>,
    pub last_find_pr_by_head_branch_name: Option<String>,
}

/// Mock implementation of GithubServiceTrait for unit tests.
///
/// # Example
/// ```rust
/// let mock = MockGithubService::new();
/// mock.state().create_draft_pr_result = Some(Ok((42, "https://github.com/...".into())));
/// // ... use in test
/// assert_eq!(mock.state().create_draft_pr_calls, 1);
/// ```
pub struct MockGithubService {
    state: Arc<Mutex<MockGithubState>>,
}

impl MockGithubService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockGithubState::default())),
        }
    }

    /// Access the inner state to configure responses or read call counts.
    pub fn state(&self) -> std::sync::MutexGuard<'_, MockGithubState> {
        self.state.lock().expect("MockGithubState lock poisoned")
    }

    /// Shorthand: configure create_draft_pr to succeed with the given values.
    pub fn will_create_pr(&self, number: i64, url: impl Into<String>) {
        self.state().create_draft_pr_result = Some(Ok((number, url.into())));
    }

    /// Shorthand: configure check_pr_status to return the given status.
    pub fn will_return_status(&self, status: PrStatus) {
        self.state().check_pr_status_result = Some(Ok(status));
    }

    /// Shorthand: configure any method to fail with the given message (Infrastructure error).
    pub fn will_fail_create_pr(&self, msg: impl Into<String>) {
        self.state().create_draft_pr_result =
            Some(Err(AppError::Infrastructure(msg.into())));
    }

    /// Shorthand: configure find_pr_by_head_branch to return the given result.
    #[allow(dead_code)]
    pub fn set_find_pr_by_head_branch(&self, result: AppResult<Option<(i64, String)>>) {
        self.state().find_pr_by_head_branch_result = Some(result);
    }
}

impl Default for MockGithubService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GithubServiceTrait for MockGithubService {
    async fn create_draft_pr(
        &self,
        _working_dir: &Path,
        base: &str,
        head: &str,
        title: &str,
        body_file: &Path,
    ) -> AppResult<(i64, String)> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.create_draft_pr_calls += 1;
        s.last_create_draft_pr_args = Some((
            base.to_string(),
            head.to_string(),
            title.to_string(),
            body_file.to_string_lossy().into_owned(),
        ));
        s.create_draft_pr_result
            .take()
            .unwrap_or(Ok((1, "https://github.com/owner/repo/pull/1".to_string())))
    }

    async fn mark_pr_ready(&self, _working_dir: &Path, pr_number: i64) -> AppResult<()> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.mark_pr_ready_calls += 1;
        s.last_mark_pr_ready_number = Some(pr_number);
        s.mark_pr_ready_result.take().unwrap_or(Ok(()))
    }

    async fn check_pr_status(&self, _working_dir: &Path, pr_number: i64) -> AppResult<PrStatus> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.check_pr_status_calls += 1;
        s.last_check_pr_status_number = Some(pr_number);
        s.check_pr_status_result
            .take()
            .unwrap_or(Ok(PrStatus::Open))
    }

    async fn push_branch(&self, _working_dir: &Path, branch: &str) -> AppResult<()> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.push_branch_calls += 1;
        s.last_push_branch_name = Some(branch.to_string());
        s.push_branch_result.take().unwrap_or(Ok(()))
    }

    async fn close_pr(&self, _working_dir: &Path, pr_number: i64) -> AppResult<()> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.close_pr_calls += 1;
        s.last_close_pr_number = Some(pr_number);
        s.close_pr_result.take().unwrap_or(Ok(()))
    }

    async fn delete_remote_branch(&self, _working_dir: &Path, branch: &str) -> AppResult<()> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.delete_remote_branch_calls += 1;
        s.last_delete_remote_branch_name = Some(branch.to_string());
        s.all_deleted_remote_branch_names.push(branch.to_string());
        s.delete_remote_branch_result.take().unwrap_or(Ok(()))
    }

    async fn fetch_remote(&self, _working_dir: &Path, branch: &str) -> AppResult<()> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.fetch_remote_calls += 1;
        s.last_fetch_remote_branch_name = Some(branch.to_string());
        s.fetch_remote_result.take().unwrap_or(Ok(()))
    }

    async fn find_pr_by_head_branch(
        &self,
        _working_dir: &Path,
        head: &str,
    ) -> AppResult<Option<(i64, String)>> {
        let mut s = self.state.lock().expect("lock poisoned");
        s.find_pr_by_head_branch_calls += 1;
        s.last_find_pr_by_head_branch_name = Some(head.to_string());
        s.find_pr_by_head_branch_result.take().unwrap_or(Ok(None))
    }
}
