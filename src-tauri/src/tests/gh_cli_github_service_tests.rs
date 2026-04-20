// Unit tests for GhCliGithubService output parsing and sanitization logic.
//
// These tests exercise the pure functions (parsers, sanitizer) without
// spawning real `gh` or `git` processes.

use crate::domain::services::github_service::PrStatus;
use crate::error::AppError;
use crate::infrastructure::services::gh_cli_github_service::{
    parse_pr_create_output, parse_pr_create_plain_output, parse_pr_status_output,
    sanitize_stderr_line, scrub_token_urls,
};

// ── parse_pr_create_output ─────────────────────────────────────────────────

#[test]
fn parse_pr_create_returns_number_and_url() {
    let json = r#"{"number": 42, "url": "https://github.com/owner/repo/pull/42"}"#;
    let (number, url) = parse_pr_create_output(json).unwrap();
    assert_eq!(number, 42);
    assert_eq!(url, "https://github.com/owner/repo/pull/42");
}

#[test]
fn parse_pr_create_fails_on_missing_number() {
    let json = r#"{"url": "https://github.com/owner/repo/pull/42"}"#;
    let err = parse_pr_create_output(json).unwrap_err();
    assert!(
        matches!(err, AppError::Infrastructure(_)),
        "Expected Infrastructure error, got: {err:?}"
    );
}

#[test]
fn parse_pr_create_fails_on_missing_url() {
    let json = r#"{"number": 42}"#;
    let err = parse_pr_create_output(json).unwrap_err();
    assert!(matches!(err, AppError::Infrastructure(_)));
}

#[test]
fn parse_pr_create_fails_on_invalid_json() {
    let err = parse_pr_create_output("not json").unwrap_err();
    assert!(matches!(err, AppError::Infrastructure(_)));
}

#[test]
fn parse_pr_create_plain_output_returns_number_and_url() {
    let stdout = "https://github.com/owner/repo/pull/42\n";
    let (number, url) = parse_pr_create_plain_output(stdout).unwrap();
    assert_eq!(number, 42);
    assert_eq!(url, "https://github.com/owner/repo/pull/42");
}

#[test]
fn parse_pr_create_plain_output_extracts_url_from_wrapped_text() {
    let stdout = "Created pull request:\n<https://github.com/owner/repo/pull/77>\n";
    let (number, url) = parse_pr_create_plain_output(stdout).unwrap();
    assert_eq!(number, 77);
    assert_eq!(url, "https://github.com/owner/repo/pull/77");
}

#[test]
fn parse_pr_create_plain_output_fails_without_url() {
    let err = parse_pr_create_plain_output("created pull request successfully")
        .unwrap_err();
    assert!(matches!(err, AppError::Infrastructure(_)));
}

// ── parse_pr_status_output ─────────────────────────────────────────────────

#[test]
fn parse_pr_status_open() {
    let json = r#"{"state": "OPEN", "mergedAt": null, "mergeCommit": null}"#;
    assert_eq!(parse_pr_status_output(json).unwrap(), PrStatus::Open);
}

#[test]
fn parse_pr_status_closed() {
    let json = r#"{"state": "CLOSED", "mergedAt": null, "mergeCommit": null}"#;
    assert_eq!(parse_pr_status_output(json).unwrap(), PrStatus::Closed);
}

#[test]
fn parse_pr_status_merged_with_sha() {
    let json = r#"{
        "state": "MERGED",
        "mergedAt": "2024-01-15T12:00:00Z",
        "mergeCommit": {"oid": "abc123def456"}
    }"#;
    let status = parse_pr_status_output(json).unwrap();
    assert_eq!(
        status,
        PrStatus::Merged {
            merge_commit_sha: Some("abc123def456".to_string())
        }
    );
}

#[test]
fn parse_pr_status_merged_without_sha() {
    let json = r#"{"state": "MERGED", "mergedAt": "2024-01-15T12:00:00Z", "mergeCommit": null}"#;
    let status = parse_pr_status_output(json).unwrap();
    assert_eq!(
        status,
        PrStatus::Merged {
            merge_commit_sha: None
        }
    );
}

#[test]
fn parse_pr_status_unknown_state_errors() {
    let json = r#"{"state": "DRAFT", "mergedAt": null, "mergeCommit": null}"#;
    let err = parse_pr_status_output(json).unwrap_err();
    assert!(matches!(err, AppError::Infrastructure(_)));
}

#[test]
fn parse_pr_status_missing_state_errors() {
    let json = r#"{"mergedAt": null}"#;
    let err = parse_pr_status_output(json).unwrap_err();
    assert!(matches!(err, AppError::Infrastructure(_)));
}

// ── sanitize_stderr_line ───────────────────────────────────────────────────

#[test]
fn sanitize_redacts_line_containing_token() {
    let line = "Error: bad token provided";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, "[REDACTED: potential secret in stderr]");
}

#[test]
fn sanitize_redacts_line_containing_bearer() {
    let line = "Authorization: Bearer ghp_abc123";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, "[REDACTED: potential secret in stderr]");
}

#[test]
fn sanitize_redacts_ghp_prefix() {
    let line = "ghp_SomeTokenValue123";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, "[REDACTED: potential secret in stderr]");
}

#[test]
fn sanitize_redacts_password_keyword() {
    let line = "Enter password:";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, "[REDACTED: potential secret in stderr]");
}

#[test]
fn sanitize_is_case_insensitive() {
    let line = "TOKEN=abc";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, "[REDACTED: potential secret in stderr]");
}

#[test]
fn sanitize_passes_through_benign_lines() {
    let line = "remote: Counting objects: 5, done.";
    let result = sanitize_stderr_line(line);
    assert_eq!(result, line);
}

// ── scrub_token_urls ───────────────────────────────────────────────────────

#[test]
fn scrub_token_urls_replaces_embedded_token() {
    let s = "Cloning into https://ghp_secret@github.com/owner/repo.git";
    let result = scrub_token_urls(s);
    assert_eq!(result, "Cloning into https://***@github.com/owner/repo.git");
}

#[test]
fn scrub_token_urls_leaves_normal_url_unchanged() {
    let s = "See https://github.com/owner/repo for details";
    let result = scrub_token_urls(s);
    assert_eq!(result, s);
}

#[test]
fn scrub_token_urls_handles_multiple_occurrences() {
    let s = "https://tok1@github.com/a and https://tok2@github.com/b";
    let result = scrub_token_urls(s);
    assert_eq!(result, "https://***@github.com/a and https://***@github.com/b");
}

#[test]
fn scrub_token_urls_no_false_positive_on_empty_token() {
    // https://@github.com — no token between :// and @
    let s = "https://@github.com/owner/repo";
    let result = scrub_token_urls(s);
    // No token present (at_pos == 0), so kept as-is
    assert_eq!(result, s);
}

#[test]
fn scrub_token_urls_no_mutation_on_plain_text() {
    let s = "Everything is fine.";
    let result = scrub_token_urls(s);
    assert_eq!(result, s);
}

// ── MockGithubService round-trip ───────────────────────────────────────────

mod mock_roundtrip {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::domain::services::github_service::{GithubServiceTrait, PrStatus};
    use crate::error::AppError;
    use crate::infrastructure::services::gh_cli_github_service::{
        GhCliCommandRunner, GhCliGithubService,
    };
    use crate::tests::mock_github_service::MockGithubService;
    use crate::AppResult;

    #[derive(Default)]
    struct MockGhCliRunner {
        gh_results: Mutex<Vec<AppResult<Vec<String>>>>,
        gh_calls: Mutex<Vec<Vec<String>>>,
        git_calls: Mutex<Vec<Vec<String>>>,
    }

    impl MockGhCliRunner {
        fn with_gh_results(results: Vec<AppResult<Vec<String>>>) -> Self {
            Self {
                gh_results: Mutex::new(results),
                gh_calls: Mutex::new(Vec::new()),
                git_calls: Mutex::new(Vec::new()),
            }
        }

        fn gh_calls(&self) -> Vec<Vec<String>> {
            self.gh_calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl GhCliCommandRunner for MockGhCliRunner {
        async fn run_gh(&self, _working_dir: &Path, args: &[String]) -> AppResult<Vec<String>> {
            self.gh_calls.lock().unwrap().push(args.to_vec());
            let mut results = self.gh_results.lock().unwrap();
            assert!(
                !results.is_empty(),
                "unexpected gh invocation with args: {:?}",
                args
            );
            results.remove(0)
        }

        async fn run_git(&self, _working_dir: &Path, args: &[String]) -> AppResult<()> {
            self.git_calls.lock().unwrap().push(args.to_vec());
            Ok(())
        }
    }

    #[tokio::test]
    async fn mock_create_draft_pr_defaults_to_pr_1() {
        let mock = MockGithubService::new();
        let (num, url) = mock
            .create_draft_pr(
                Path::new("/tmp"),
                "main",
                "feature",
                "Test PR",
                Path::new("/tmp/body.md"),
            )
            .await
            .unwrap();
        assert_eq!(num, 1);
        assert!(url.contains("pull/1"));
        assert_eq!(mock.state().create_draft_pr_calls, 1);
    }

    #[tokio::test]
    async fn mock_create_draft_pr_configurable() {
        let mock = MockGithubService::new();
        mock.will_create_pr(99, "https://github.com/a/b/pull/99");

        let (num, url) = mock
            .create_draft_pr(
                Path::new("/tmp"),
                "main",
                "feat",
                "My PR",
                Path::new("/tmp/body.md"),
            )
            .await
            .unwrap();

        assert_eq!(num, 99);
        assert_eq!(url, "https://github.com/a/b/pull/99");
        assert_eq!(mock.state().create_draft_pr_calls, 1);
    }

    #[tokio::test]
    async fn mock_check_pr_status_configurable() {
        let mock = MockGithubService::new();
        mock.will_return_status(PrStatus::Merged {
            merge_commit_sha: Some("deadbeef".to_string()),
        });

        let status = mock
            .check_pr_status(Path::new("/tmp"), 42)
            .await
            .unwrap();

        assert_eq!(
            status,
            PrStatus::Merged {
                merge_commit_sha: Some("deadbeef".to_string())
            }
        );
        assert_eq!(mock.state().check_pr_status_calls, 1);
        assert_eq!(mock.state().last_check_pr_status_number, Some(42));
    }

    #[tokio::test]
    async fn mock_tracks_all_calls() {
        let mock = MockGithubService::new();
        let p = Path::new("/tmp");

        mock.push_branch(p, "feat/foo").await.unwrap();
        mock.fetch_remote(p, "main").await.unwrap();
        mock.mark_pr_ready(p, 7).await.unwrap();
        mock.close_pr(p, 7).await.unwrap();
        mock.delete_remote_branch(p, "feat/foo").await.unwrap();

        let s = mock.state();
        assert_eq!(s.push_branch_calls, 1);
        assert_eq!(s.fetch_remote_calls, 1);
        assert_eq!(s.mark_pr_ready_calls, 1);
        assert_eq!(s.close_pr_calls, 1);
        assert_eq!(s.delete_remote_branch_calls, 1);
        assert_eq!(s.last_push_branch_name.as_deref(), Some("feat/foo"));
        assert_eq!(s.last_fetch_remote_branch_name.as_deref(), Some("main"));
        assert_eq!(s.last_mark_pr_ready_number, Some(7));
        assert_eq!(s.last_close_pr_number, Some(7));
        assert_eq!(s.last_delete_remote_branch_name.as_deref(), Some("feat/foo"));
    }

    #[tokio::test]
    async fn mock_error_propagated() {
        let mock = MockGithubService::new();
        mock.will_fail_create_pr("gh: not authenticated");

        let err = mock
            .create_draft_pr(
                Path::new("/tmp"),
                "main",
                "feat",
                "PR",
                Path::new("/tmp/b.md"),
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("not authenticated"));
    }

    #[tokio::test]
    async fn create_draft_pr_falls_back_when_create_json_flag_is_unsupported() {
        let runner = Arc::new(MockGhCliRunner::with_gh_results(vec![
            Err(AppError::Infrastructure(
                "gh exited with code 1: unknown flag: --json".to_string(),
            )),
            Ok(vec!["https://github.com/owner/repo/pull/42".to_string()]),
        ]));
        let service = GhCliGithubService::with_runner(runner.clone());

        let (number, url) = service
            .create_draft_pr(
                Path::new("/tmp"),
                "main",
                "feature/pr-mode-fallback",
                "Compatibility PR",
                Path::new("/tmp/body.md"),
            )
            .await
            .unwrap();

        assert_eq!(number, 42);
        assert_eq!(url, "https://github.com/owner/repo/pull/42");

        let calls = runner.gh_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0],
            vec![
                "pr",
                "create",
                "--draft",
                "--base",
                "main",
                "--head",
                "feature/pr-mode-fallback",
                "--title",
                "Compatibility PR",
                "--body-file",
                "/tmp/body.md",
                "--json",
                "number,url",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
        assert_eq!(
            calls[1],
            vec![
                "pr",
                "create",
                "--draft",
                "--base",
                "main",
                "--head",
                "feature/pr-mode-fallback",
                "--title",
                "Compatibility PR",
                "--body-file",
                "/tmp/body.md",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn create_draft_pr_preserves_duplicate_error_on_plain_fallback() {
        let runner = Arc::new(MockGhCliRunner::with_gh_results(vec![
            Err(AppError::Infrastructure(
                "gh exited with code 1: unknown flag: --json".to_string(),
            )),
            Err(AppError::Infrastructure(
                "gh exited with code 1: a pull request for branch \"feature/pr-mode-fallback\" already exists".to_string(),
            )),
        ]));
        let service = GhCliGithubService::with_runner(runner.clone());

        let err = service
            .create_draft_pr(
                Path::new("/tmp"),
                "main",
                "feature/pr-mode-fallback",
                "Compatibility PR",
                Path::new("/tmp/body.md"),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::DuplicatePr));
        assert_eq!(runner.gh_calls().len(), 2);
    }
}
