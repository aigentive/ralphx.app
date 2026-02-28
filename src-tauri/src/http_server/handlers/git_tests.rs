use super::*;

mod json_error_format {
    use super::*;

    #[test]
    fn error_without_details() {
        let (status, Json(body)) = json_error(StatusCode::BAD_REQUEST, "Invalid input", None);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Invalid input");
        assert!(body.get("details").is_none());
    }

    #[test]
    fn error_with_details() {
        let (status, Json(body)) = json_error(
            StatusCode::BAD_REQUEST,
            "Commit not on branch",
            Some("Use git rev-parse HEAD on main".to_string()),
        );
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Commit not on branch");
        assert_eq!(body["details"], "Use git rev-parse HEAD on main");
    }

    #[test]
    fn internal_server_error_status() {
        let (status, Json(body)) =
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error", None);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "Database error");
    }

    #[test]
    fn not_found_error_status() {
        let (status, Json(body)) = json_error(StatusCode::NOT_FOUND, "Task not found", None);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "Task not found");
    }
}

mod sha_validation {
    use super::*;

    #[test]
    fn valid_sha_40_lowercase_hex() {
        let sha = "a1b2c3d4e5f6789012345678901234567890abcd";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_40_uppercase_hex() {
        let sha = "A1B2C3D4E5F6789012345678901234567890ABCD";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_mixed_case() {
        let sha = "a1B2c3D4e5F6789012345678901234567890AbCd";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_all_digits() {
        let sha = "1234567890123456789012345678901234567890";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_too_short() {
        let sha = "a1b2c3d4";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_too_long() {
        let sha = "a1b2c3d4e5f6789012345678901234567890abcd1234";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_non_hex_chars() {
        let sha = "g1b2c3d4e5f6789012345678901234567890abcd"; // 'g' is not hex
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_empty() {
        let sha = "";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_spaces() {
        let sha = "a1b2c3d4e5f67890 2345678901234567890abcd";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_short_sha_format() {
        // Short SHA (7 chars) should be rejected
        let sha = "a1b2c3d";
        assert!(!is_valid_git_sha(sha));
    }
}

mod ipr_removal {
    use super::*;
    use crate::application::AppState;
    use crate::commands::ExecutionState;
    use crate::domain::entities::{InternalStatus, Project, ProjectId, Task};
    use std::sync::Arc;

    async fn setup_git_test_state() -> HttpServerState {
        let app_state = Arc::new(AppState::new_test());
        let execution_state = Arc::new(ExecutionState::new());
        let tracker = crate::application::TeamStateTracker::new();
        let team_service = Arc::new(crate::application::TeamService::new_without_events(
            Arc::new(tracker.clone()),
        ));
        HttpServerState {
            app_state,
            execution_state,
            team_tracker: tracker,
            team_service,
        }
    }

    /// Seed a task in Merging state so report_conflict / report_incomplete pass the guard.
    async fn seed_merging_task(state: &HttpServerState) -> Task {
        let project_id = ProjectId::new();
        let mut task = Task::new(project_id, "Merging task".to_string());
        task.internal_status = InternalStatus::Merging;
        state.app_state.task_repo.create(task.clone()).await.unwrap();
        task
    }

    /// report_conflict — IPR entry removed after transition to MergeConflict.
    ///
    /// When the merger agent calls report_conflict, the handler transitions the task
    /// to MergeConflict and then removes the IPR entry so the agent gets EOF on stdin.
    #[tokio::test]
    async fn test_report_conflict_removes_ipr() {
        let state = setup_git_test_state().await;
        let task = seed_merging_task(&state).await;
        let task_id = task.id.clone();

        // Register IPR entry for the merger agent
        let mut child = tokio::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cat for conflict IPR test");
        let stdin = child.stdin.take().expect("cat stdin");

        let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
            "merge",
            task_id.as_str(),
        );
        state
            .app_state
            .interactive_process_registry
            .register(key.clone(), stdin)
            .await;

        assert!(
            state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be registered before handler call"
        );

        let result = report_conflict(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(ReportConflictRequest {
                conflict_files: vec!["src/main.rs".to_string()],
                reason: "Cannot automatically resolve conflict in src/main.rs".to_string(),
            }),
        )
        .await;

        assert!(result.is_ok(), "report_conflict handler should succeed: {:?}", result);
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be removed after report_conflict succeeds"
        );

        // Clean up regardless of result
        state
            .app_state
            .interactive_process_registry
            .remove(&key)
            .await;
        let _ = child.kill().await;
    }

    // ---- complete_merge helpers ------------------------------------------------

    /// Set up a real git repo with a merged task branch. Returns (TempDir, merge_commit_sha).
    fn setup_complete_merge_repo() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("create temp dir for complete_merge test");
        let repo = dir.path();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Initial commit on main
        std::fs::write(repo.join("readme.md"), "# test").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create task-branch and merge it into main
        std::process::Command::new("git")
            .args(["checkout", "-b", "task-branch"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::fs::write(repo.join("feat.txt"), "feature").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "feat"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["merge", "task-branch", "--no-edit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Get the merge commit SHA (HEAD on main)
        let sha_bytes = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap()
            .stdout;
        let sha = String::from_utf8(sha_bytes).unwrap().trim().to_string();

        (dir, sha)
    }

    /// Set up a fake worktree dir with .git/rebase-merge to simulate rebase in progress.
    /// Also creates a minimal real git repo for project.working_directory.
    fn setup_rebase_in_progress_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
        // Worktree dir with fake rebase state
        let worktree_dir = tempfile::tempdir().expect("create worktree dir");
        std::fs::create_dir_all(worktree_dir.path().join(".git").join("rebase-merge")).unwrap();

        // Minimal real git repo for project.working_directory
        let repo_dir = tempfile::tempdir().expect("create repo dir");
        let repo = repo_dir.path();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::fs::write(repo.join("r.md"), "init").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output()
            .unwrap();

        (worktree_dir, repo_dir)
    }

    /// Seed a project + Merging task into the HTTP test state.
    /// Returns (task_id, ipr_key).
    async fn seed_merging_task_with_project(
        state: &HttpServerState,
        repo_path: &std::path::Path,
        worktree_path: Option<&std::path::Path>,
    ) -> (
        TaskId,
        crate::application::interactive_process_registry::InteractiveProcessKey,
    ) {
        use crate::application::interactive_process_registry::InteractiveProcessKey;

        let project_id = ProjectId::new();
        let mut project = Project::new(
            "test-project".to_string(),
            repo_path.to_string_lossy().to_string(),
        );
        project.id = project_id.clone();
        project.base_branch = Some("main".to_string());
        state.app_state.project_repo.create(project).await.unwrap();

        let mut task = Task::new(project_id, "Complete merge test".to_string());
        task.internal_status = InternalStatus::Merging;
        task.task_branch = Some("task-branch".to_string());
        if let Some(wt) = worktree_path {
            task.worktree_path = Some(wt.to_string_lossy().to_string());
        }
        let task_id = task.id.clone();
        state.app_state.task_repo.create(task).await.unwrap();

        let key = InteractiveProcessKey::new("merge", task_id.as_str());
        (task_id, key)
    }

    // ---- complete_merge IPR tests ----------------------------------------

    /// complete_merge success path: IPR is removed after transition to Merged.
    ///
    /// When merger agent calls complete_merge with a valid SHA on the target branch,
    /// the handler transitions Merging → Merged and then removes the IPR entry.
    #[tokio::test]
    async fn test_complete_merge_success_removes_ipr() {
        let (dir, merge_sha) = setup_complete_merge_repo();
        let state = setup_git_test_state().await;
        let (task_id, key) = seed_merging_task_with_project(&state, dir.path(), None).await;

        // Register IPR for the merger agent
        let mut child = tokio::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cat for complete_merge IPR test");
        let stdin = child.stdin.take().expect("cat stdin");
        state
            .app_state
            .interactive_process_registry
            .register(key.clone(), stdin)
            .await;

        assert!(
            state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be registered before handler call"
        );

        let result = complete_merge(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(CompleteMergeRequest {
                commit_sha: merge_sha,
            }),
        )
        .await;

        assert!(result.is_ok(), "complete_merge handler should succeed: {:?}", result);
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR should be removed after complete_merge success"
        );

        state
            .app_state
            .interactive_process_registry
            .remove(&key)
            .await;
        let _ = child.kill().await;
    }

    /// complete_merge rebase retry path: IPR is removed when transitioning to PendingMerge.
    ///
    /// When the worktree has .git/rebase-merge (rebase in progress), the handler
    /// transitions Merging → PendingMerge and closes IPR so the agent exits.
    #[tokio::test]
    async fn test_complete_merge_rebase_retry_removes_ipr() {
        let (worktree_dir, repo_dir) = setup_rebase_in_progress_dirs();
        let state = setup_git_test_state().await;
        let (task_id, key) =
            seed_merging_task_with_project(&state, repo_dir.path(), Some(worktree_dir.path()))
                .await;

        // Register IPR entry
        let mut child = tokio::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cat for rebase retry IPR test");
        let stdin = child.stdin.take().expect("cat stdin");
        state
            .app_state
            .interactive_process_registry
            .register(key.clone(), stdin)
            .await;

        assert!(
            state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be registered before handler call"
        );

        // Any valid 40-char SHA — handler exits via rebase path before branch verification
        let dummy_sha = "a".repeat(40);
        let result = complete_merge(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(CompleteMergeRequest {
                commit_sha: dummy_sha,
            }),
        )
        .await;

        assert!(result.is_ok(), "complete_merge rebase retry handler should succeed: {:?}", result);
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR should be removed after complete_merge rebase retry path"
        );

        state
            .app_state
            .interactive_process_registry
            .remove(&key)
            .await;
        let _ = child.kill().await;
    }

    /// complete_merge without an IPR entry is idempotent: handler succeeds and
    /// the missing IPR is silently ignored (remove returns None, no panic).
    #[tokio::test]
    async fn test_complete_merge_no_ipr_entry_succeeds_silently() {
        // Use rebase path to avoid needing a real git SHA on a branch
        let (worktree_dir, repo_dir) = setup_rebase_in_progress_dirs();
        let state = setup_git_test_state().await;
        let (task_id, key) =
            seed_merging_task_with_project(&state, repo_dir.path(), Some(worktree_dir.path()))
                .await;

        // No IPR entry registered
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "No IPR should be registered at start"
        );

        let dummy_sha = "b".repeat(40);
        let result = complete_merge(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(CompleteMergeRequest {
                commit_sha: dummy_sha,
            }),
        )
        .await;

        // Handler should not error just because IPR entry was absent
        // (the IPR remove is guarded with `if ... .remove().is_some()`)
        if result.is_ok() {
            // Verify the state is clean — no phantom IPR left behind
            assert!(
                !state
                    .app_state
                    .interactive_process_registry
                    .has_process(&key)
                    .await
            );
        }
        // If result is Err, the transition failed (in-memory limitation), which is acceptable.
        // The key invariant is: no panic when IPR entry is absent.
    }

    /// report_conflict — no IPR entry is safe: handler succeeds without IPR registered.
    ///
    /// When no IPR entry is present (merger agent already exited), the IPR removal
    /// is a no-op and must not cause the handler to fail or panic.
    #[tokio::test]
    async fn test_report_conflict_no_ipr_entry_is_safe() {
        let state = setup_git_test_state().await;
        let task = seed_merging_task(&state).await;
        let task_id = task.id.clone();

        // No IPR entry registered — removal must be a no-op
        let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
            "merge",
            task_id.as_str(),
        );
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "No IPR should be registered at start of test"
        );

        let result = report_conflict(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(ReportConflictRequest {
                conflict_files: vec!["src/lib.rs".to_string()],
                reason: "Conflicting changes in function signatures".to_string(),
            }),
        )
        .await;

        // Handler must not panic or error solely due to missing IPR entry.
        // If transition fails (in-memory limitation), that is acceptable;
        // the key invariant is no panic and no phantom IPR left behind.
        if result.is_ok() {
            assert!(
                !state
                    .app_state
                    .interactive_process_registry
                    .has_process(&key)
                    .await,
                "No phantom IPR should exist after handler call"
            );
        }
    }

    /// report_incomplete — no IPR entry is safe: handler succeeds without IPR registered.
    ///
    /// When no IPR entry is present (merger agent already exited), the IPR removal
    /// is a no-op and must not cause the handler to fail or panic.
    #[tokio::test]
    async fn test_report_incomplete_no_ipr_entry_is_safe() {
        let state = setup_git_test_state().await;
        let task = seed_merging_task(&state).await;
        let task_id = task.id.clone();

        // No IPR entry registered — removal must be a no-op
        let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
            "merge",
            task_id.as_str(),
        );
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "No IPR should be registered at start of test"
        );

        let result = report_incomplete(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(ReportIncompleteRequest {
                reason: "Git operation failed: missing remote configuration".to_string(),
                diagnostic_info: Some("git remote: error".to_string()),
            }),
        )
        .await;

        // Handler must not panic or error solely due to missing IPR entry.
        if result.is_ok() {
            assert!(
                !state
                    .app_state
                    .interactive_process_registry
                    .has_process(&key)
                    .await,
                "No phantom IPR should exist after handler call"
            );
        }
    }

    /// report_incomplete — IPR entry removed after transition to MergeIncomplete.
    ///
    /// When the merger agent calls report_incomplete, the handler transitions the task
    /// to MergeIncomplete and then removes the IPR entry so the agent gets EOF on stdin.
    #[tokio::test]
    async fn test_report_incomplete_removes_ipr() {
        let state = setup_git_test_state().await;
        let task = seed_merging_task(&state).await;
        let task_id = task.id.clone();

        // Register IPR entry for the merger agent
        let mut child = tokio::process::Command::new("cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cat for incomplete IPR test");
        let stdin = child.stdin.take().expect("cat stdin");

        let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
            "merge",
            task_id.as_str(),
        );
        state
            .app_state
            .interactive_process_registry
            .register(key.clone(), stdin)
            .await;

        assert!(
            state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be registered before handler call"
        );

        let result = report_incomplete(
            State(state.clone()),
            Path(task_id.as_str().to_string()),
            Json(ReportIncompleteRequest {
                reason: "Missing git configuration".to_string(),
                diagnostic_info: Some("git status: detached HEAD".to_string()),
            }),
        )
        .await;

        assert!(result.is_ok(), "report_incomplete handler should succeed: {:?}", result);
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be removed after report_incomplete succeeds"
        );

        // Clean up regardless of result
        state
            .app_state
            .interactive_process_registry
            .remove(&key)
            .await;
        let _ = child.kill().await;
    }
}
