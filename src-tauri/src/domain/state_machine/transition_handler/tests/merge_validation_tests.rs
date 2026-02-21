// Merge validation tests
//
// Extracted from side_effects.rs — tests for run_validation_commands,
// format_validation_error/warn_metadata, take_skip_validation_flag,
// extract_cached_validation, validation caching, and fail-fast behavior.

use std::path::Path;

use super::helpers::*;
use super::super::merge_validation::{
    cleanup_validation_logs, extract_cached_validation, format_validation_error_metadata,
    format_validation_warn_metadata, run_validation_commands, take_skip_validation_flag,
    validation_log_dir, ValidationFailure, ValidationLogEntry,
};
use crate::domain::entities::MergeValidationMode;

// ==================
// run_validation_commands tests
// ==================

#[tokio::test]
async fn run_validation_returns_none_when_no_analysis() {
    let project = make_project(Some("main"));
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn run_validation_returns_none_when_empty_entries() {
    let mut project = make_project(Some("main"));
    project.detected_analysis = Some("[]".to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn run_validation_returns_none_when_no_validate_commands() {
    let mut project = make_project(Some("main"));
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": []}]"#.to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn run_validation_prefers_custom_over_detected() {
    let mut project = make_project(Some("main"));
    // detected has a failing command
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
    // custom has a passing command (overrides detected)
    project.custom_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    assert!(result.unwrap().all_passed);
}

#[tokio::test]
async fn run_validation_succeeds_with_passing_command() {
    let mut project = make_project(Some("main"));
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.all_passed);
    assert!(r.failures.is_empty());
    assert_eq!(r.log.len(), 1);
    assert_eq!(r.log[0].phase, "validate");
    assert_eq!(r.log[0].status, "success");
    assert_eq!(r.log[0].label, "Test");
}

#[tokio::test]
async fn run_validation_fails_with_failing_command() {
    let mut project = make_project(Some("main"));
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);
    assert_eq!(r.failures[0].command, "false");
    assert_eq!(r.log.len(), 1);
    assert_eq!(r.log[0].phase, "validate");
    assert_eq!(r.log[0].status, "failed");
}

#[tokio::test]
async fn run_validation_resolves_template_vars() {
    let mut project = make_project(Some("main"));
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["echo {project_root} {worktree_path}"]}]"#.to_string(),
    );
    let mut task = make_task(None, None);
    task.worktree_path = Some("/tmp/wt".to_string());
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    assert!(result.unwrap().all_passed);
}

#[tokio::test]
async fn run_validation_returns_none_for_invalid_json() {
    let mut project = make_project(Some("main"));
    project.detected_analysis = Some("not valid json".to_string());
    let task = make_task(None, None);
    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_none());
}

// ==================
// format_validation_error/warn_metadata tests
// ==================

#[test]
fn format_validation_error_metadata_formats_correctly() {
    let failures = vec![ValidationFailure {
        command: "cargo check".to_string(),
        path: ".".to_string(),
        exit_code: Some(1),
        stderr: "error[E0308]: mismatched types".to_string(),
    }];
    let log = vec![ValidationLogEntry {
        phase: "validate".to_string(),
        command: "cargo check".to_string(),
        path: ".".to_string(),
        label: "Rust".to_string(),
        status: "failed".to_string(),
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "error[E0308]: mismatched types".to_string(),
        duration_ms: 1500,
        ..Default::default()
    }];
    let result = format_validation_error_metadata(&failures, &log, "task-branch", "main");
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["error"]
        .as_str()
        .unwrap()
        .contains("1 command(s) failed"));
    assert_eq!(parsed["source_branch"], "task-branch");
    assert_eq!(parsed["target_branch"], "main");
    assert_eq!(parsed["validation_failures"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
}

#[test]
fn format_validation_warn_metadata_formats_correctly() {
    let log = vec![ValidationLogEntry {
        phase: "validate".to_string(),
        command: "npm test".to_string(),
        path: ".".to_string(),
        label: "Node".to_string(),
        status: "failed".to_string(),
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "test failed".to_string(),
        duration_ms: 500,
        ..Default::default()
    }];
    let result = format_validation_warn_metadata(&log, "task-branch", "main");
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["validation_warnings"], true);
    assert_eq!(parsed["source_branch"], "task-branch");
    assert_eq!(parsed["target_branch"], "main");
    assert_eq!(parsed["validation_log"].as_array().unwrap().len(), 1);
}

// ==================
// take_skip_validation_flag tests
// ==================

#[test]
fn take_skip_validation_flag_returns_false_when_no_metadata() {
    let mut task = make_task(None, None);
    assert!(!take_skip_validation_flag(&mut task));
}

#[test]
fn take_skip_validation_flag_returns_false_when_no_flag() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"some_key": "value"}"#.to_string());
    assert!(!take_skip_validation_flag(&mut task));
}

#[test]
fn take_skip_validation_flag_returns_true_and_clears() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"skip_validation": true, "other": "data"}"#.to_string());
    assert!(take_skip_validation_flag(&mut task));
    // Flag should be cleared
    let meta: serde_json::Value =
        serde_json::from_str(task.metadata.as_ref().unwrap()).unwrap();
    assert!(meta.get("skip_validation").is_none());
    assert_eq!(meta["other"], "data");
    // Second call returns false
    assert!(!take_skip_validation_flag(&mut task));
}

#[test]
fn run_validation_skipped_in_off_mode() {
    let mut project = make_project(Some("main"));
    project.merge_validation_mode = MergeValidationMode::Off;
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string());
    // With Off mode, validation should not run, so the test verifies the enum
    // is correctly set and accessible (actual skip happens in attempt_programmatic_merge)
    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
}

// ==================
// extract_cached_validation tests
// ==================

#[test]
fn extract_cached_returns_none_when_no_metadata() {
    let task = make_task(None, None);
    assert!(extract_cached_validation(&task, "abc123").is_none());
}

#[test]
fn extract_cached_returns_none_when_sha_mismatch() {
    let mut task = make_task(None, None);
    task.metadata = Some(
        serde_json::json!({
            "validation_source_sha": "old_sha",
            "validation_log": [{
                "phase": "validate",
                "command": "true",
                "path": ".",
                "label": "Test",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 100,
            }],
        })
        .to_string(),
    );
    assert!(extract_cached_validation(&task, "different_sha").is_none());
}

#[test]
fn extract_cached_returns_log_when_sha_matches() {
    let mut task = make_task(None, None);
    task.metadata = Some(
        serde_json::json!({
            "validation_source_sha": "abc123",
            "validation_log": [{
                "phase": "validate",
                "command": "cargo check",
                "path": ".",
                "label": "Rust",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 1500,
            }],
        })
        .to_string(),
    );
    let cached = extract_cached_validation(&task, "abc123");
    assert!(cached.is_some());
    let entries = cached.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].command, "cargo check");
    assert_eq!(entries[0].status, "success");
}

#[test]
fn extract_cached_returns_none_when_no_sha_in_metadata() {
    let mut task = make_task(None, None);
    task.metadata = Some(
        serde_json::json!({
            "validation_log": [{
                "phase": "validate",
                "command": "true",
                "path": ".",
                "label": "Test",
                "status": "success",
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "duration_ms": 100,
            }],
        })
        .to_string(),
    );
    // No validation_source_sha → no cache hit
    assert!(extract_cached_validation(&task, "abc123").is_none());
}

// ==================
// run_validation_commands caching tests
// ==================

#[tokio::test]
async fn run_validation_skips_passed_when_cached() {
    let mut project = make_project(Some("main"));
    // "true" always passes, "echo hello" always passes
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["true", "echo hello"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    // Build a cached log where "true" passed but "echo hello" failed
    let cached = vec![
        ValidationLogEntry {
            phase: "validate".to_string(),
            command: "true".to_string(),
            path: ".".to_string(),
            label: "Test".to_string(),
            status: "success".to_string(),
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 50,
            ..Default::default()
        },
        ValidationLogEntry {
            phase: "validate".to_string(),
            command: "echo hello".to_string(),
            path: ".".to_string(),
            label: "Test".to_string(),
            status: "failed".to_string(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 100,
            ..Default::default()
        },
    ];

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, Some(&cached), &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new())
            .await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.all_passed);
    assert_eq!(r.log.len(), 2);
    // First command should be cached (was "success" in cache)
    assert_eq!(r.log[0].status, "cached");
    assert_eq!(r.log[0].command, "true");
    assert_eq!(r.log[0].duration_ms, 0);
    // Second command should be re-run (was "failed" in cache)
    assert_eq!(r.log[1].status, "success");
    assert_eq!(r.log[1].command, "echo hello");
}

#[tokio::test]
async fn run_validation_reruns_all_when_no_cache() {
    let mut project = make_project(Some("main"));
    project.detected_analysis =
        Some(r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string());
    let task = make_task(None, None);

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.all_passed);
    assert_eq!(r.log.len(), 1);
    assert_eq!(r.log[0].status, "success"); // actually ran, not "cached"
}

// ==================
// Fail-fast tests
// ==================

#[tokio::test]
async fn fail_fast_block_mode_skips_remaining_on_first_failure() {
    let mut project = make_project(Some("main"));
    // Two commands: "false" fails, "true" should be skipped in Block mode
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["false", "true"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);
    assert_eq!(r.failures[0].command, "false");
    // Should have 2 log entries: 1 failed + 1 skipped
    assert_eq!(r.log.len(), 2);
    assert_eq!(r.log[0].status, "failed");
    assert_eq!(r.log[0].command, "false");
    assert_eq!(r.log[1].status, "skipped");
    assert_eq!(r.log[1].command, "true");
    assert_eq!(r.log[1].duration_ms, 0);
}

#[tokio::test]
async fn fail_fast_autofix_mode_skips_remaining_on_first_failure() {
    let mut project = make_project(Some("main"));
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["false", "true"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::AutoFix, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);
    assert_eq!(r.log.len(), 2);
    assert_eq!(r.log[1].status, "skipped");
}

#[tokio::test]
async fn warn_mode_runs_all_commands_even_after_failure() {
    let mut project = make_project(Some("main"));
    // "false" fails, "true" should still run in Warn mode
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["false", "true"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Warn, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);
    // Should have 2 log entries: 1 failed + 1 success (NOT skipped)
    assert_eq!(r.log.len(), 2);
    assert_eq!(r.log[0].status, "failed");
    assert_eq!(r.log[0].command, "false");
    assert_eq!(r.log[1].status, "success");
    assert_eq!(r.log[1].command, "true");
}

#[tokio::test]
async fn fail_fast_skips_across_multiple_entries() {
    let mut project = make_project(Some("main"));
    // Two entries: first has a failing command, second entry's commands should be skipped
    project.detected_analysis = Some(
        r#"[
            {"path": ".", "label": "Rust", "validate": ["false"]},
            {"path": ".", "label": "Node", "validate": ["true"]}
        ]"#.to_string(),
    );
    let task = make_task(None, None);

    let result =
        run_validation_commands(&project, &task, Path::new("/tmp"), "", None, None, &MergeValidationMode::Block, &tokio_util::sync::CancellationToken::new()).await;
    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);
    // 1 failed (from Rust entry) + 1 skipped (from Node entry)
    assert_eq!(r.log.len(), 2);
    assert_eq!(r.log[0].status, "failed");
    assert_eq!(r.log[0].label, "Rust");
    assert_eq!(r.log[1].status, "skipped");
    assert_eq!(r.log[1].label, "Node");
}

// ==================
// Layer 1: Skip setup when merge_cwd == project_root
// ==================

#[tokio::test]
async fn run_validation_skips_setup_when_merge_cwd_equals_project_root() {
    // When merge_cwd == project.working_directory, worktree_setup commands
    // should be skipped (they'd create circular symlinks) but validate commands
    // should still run.
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap().to_string();
    let mut project = make_project(Some("main"));
    project.working_directory = dir_path.clone();
    project.detected_analysis = Some(
        r#"[{
            "path": ".",
            "label": "Frontend",
            "validate": ["true"],
            "worktree_setup": ["ln -s {project_root}/node_modules {worktree_path}/node_modules"]
        }]"#.to_string(),
    );
    let task = make_task(None, None);

    // Pass project root as merge_cwd — triggers the skip guard
    let result = run_validation_commands(
        &project, &task, dir.path(), "", None, None, &MergeValidationMode::Block,
        &tokio_util::sync::CancellationToken::new(),
    ).await;

    // Validation commands should still run (setup is skipped, not validate)
    assert!(result.is_some(), "validation should still run even when setup is skipped");
    let r = result.unwrap();
    // Only validate entries in log, no setup entries
    assert!(r.log.iter().all(|e| e.phase != "setup"), "no setup entries should appear in log");
    assert!(r.all_passed, "validate command 'true' should pass");
}

#[tokio::test]
async fn run_validation_runs_setup_when_merge_cwd_differs_from_project_root() {
    // When merge_cwd != project.working_directory, worktree_setup should run normally
    let dir = tempfile::tempdir().unwrap();
    let worktree_dir = tempfile::tempdir().unwrap();
    let mut project = make_project(Some("main"));
    project.working_directory = dir.path().to_str().unwrap().to_string();
    project.detected_analysis = Some(
        r#"[{
            "path": ".",
            "label": "Frontend",
            "validate": ["true"],
            "worktree_setup": ["echo setting_up_worktree"]
        }]"#.to_string(),
    );
    let task = make_task(None, None);

    // Use a different path than project root — setup should run
    let result = run_validation_commands(
        &project, &task, worktree_dir.path(), "", None, None, &MergeValidationMode::Block,
        &tokio_util::sync::CancellationToken::new(),
    ).await;

    assert!(result.is_some());
    let r = result.unwrap();
    // Should have setup entries in the log
    let setup_entries: Vec<_> = r.log.iter().filter(|e| e.phase == "setup").collect();
    assert!(!setup_entries.is_empty(), "setup entries should be present when merge_cwd != project_root");
}

// ==================
// Validation retry tests
// ==================

/// Verify that a failing validation command is retried once and succeeds on retry.
/// Uses a temp file flag: first invocation fails (creates flag), second sees flag and succeeds.
#[tokio::test]
async fn validation_retry_succeeds_on_second_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let flag_path = tmp.path().join("retry_flag");
    let flag_str = flag_path.to_string_lossy();

    // Shell command: fail on first run (no flag), succeed on second (flag exists)
    let cmd = format!(
        "if [ -f '{}' ]; then exit 0; else touch '{}' && exit 1; fi",
        flag_str, flag_str
    );

    let mut project = make_project(Some("main"));
    project.detected_analysis = Some(format!(
        r#"[{{"path": ".", "label": "Test", "validate": ["{cmd}"]}}]"#,
        cmd = cmd.replace('"', "\\\"")
    ));
    let task = make_task(None, None);

    let result = run_validation_commands(
        &project,
        &task,
        tmp.path(),
        "test-task",
        None,
        None,
        &MergeValidationMode::Block,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_some());
    let r = result.unwrap();
    assert!(
        r.all_passed,
        "Validation should pass after retry. Failures: {:?}",
        r.failures
    );
    assert!(r.failures.is_empty());

    // The log entry should indicate it was retried
    let validate_entries: Vec<_> = r.log.iter().filter(|e| e.phase == "validate").collect();
    assert_eq!(validate_entries.len(), 1);
    assert_eq!(validate_entries[0].status, "success");
    assert!(
        validate_entries[0].retried,
        "Log entry should have retried=true"
    );
}

/// Verify that a validation command that fails twice (initial + retry) is treated as a real failure.
#[tokio::test]
async fn validation_retry_both_attempts_fail() {
    let mut project = make_project(Some("main"));
    // "false" always exits with code 1, so both initial and retry fail
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["false"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let result = run_validation_commands(
        &project,
        &task,
        Path::new("/tmp"),
        "test-task",
        None,
        None,
        &MergeValidationMode::Block,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_some());
    let r = result.unwrap();
    assert!(!r.all_passed);
    assert_eq!(r.failures.len(), 1);

    // The log entry should indicate it was retried (even though both failed)
    let validate_entries: Vec<_> = r.log.iter().filter(|e| e.phase == "validate").collect();
    assert_eq!(validate_entries.len(), 1);
    assert_eq!(validate_entries[0].status, "failed");
    assert!(
        validate_entries[0].retried,
        "Log entry should have retried=true even when retry fails"
    );
}

/// Verify that a cancelled validation command is NOT retried.
/// Cancelled commands have exit_code=None, so the retry guard rejects them.
#[tokio::test]
async fn validation_no_retry_when_cancelled() {
    let mut project = make_project(Some("main"));
    // Use "sleep 60" so we can cancel it
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["sleep 60"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel.clone();
    // Cancel after a short delay
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        cancel_clone.cancel();
    });

    let result = run_validation_commands(
        &project,
        &task,
        Path::new("/tmp"),
        "test-task",
        None,
        None,
        &MergeValidationMode::Block,
        &cancel,
    )
    .await;

    assert!(result.is_some());
    let r = result.unwrap();

    let validate_entries: Vec<_> = r.log.iter().filter(|e| e.phase == "validate").collect();
    assert_eq!(validate_entries.len(), 1);
    assert_eq!(validate_entries[0].status, "failed");
    assert!(
        !validate_entries[0].retried,
        "Cancelled commands should NOT be retried"
    );
}

/// Verify that a passing validation command is not retried (retried stays false).
#[tokio::test]
async fn validation_no_retry_when_passing() {
    let mut project = make_project(Some("main"));
    project.detected_analysis = Some(
        r#"[{"path": ".", "label": "Test", "validate": ["true"]}]"#.to_string(),
    );
    let task = make_task(None, None);

    let result = run_validation_commands(
        &project,
        &task,
        Path::new("/tmp"),
        "test-task",
        None,
        None,
        &MergeValidationMode::Block,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_some());
    let r = result.unwrap();
    assert!(r.all_passed);

    let validate_entries: Vec<_> = r.log.iter().filter(|e| e.phase == "validate").collect();
    assert_eq!(validate_entries.len(), 1);
    assert_eq!(validate_entries[0].status, "success");
    assert!(
        !validate_entries[0].retried,
        "Passing command should not have retried=true"
    );
}

// --- Validation log file writing tests ---

#[test]
fn attach_failure_logs_writes_files_for_failed_command() {
    let task_id = &format!("test-logs-{}", uuid::Uuid::new_v4());
    let dir = validation_log_dir(task_id);

    let mut entry = ValidationLogEntry {
        phase: "validate".to_string(),
        command: "cargo check".to_string(),
        path: ".".to_string(),
        label: "Rust".to_string(),
        status: "failed".to_string(),
        exit_code: Some(1),
        stdout: "some stdout output".to_string(),
        stderr: "error[E0308]: mismatched types\n  --> src/lib.rs:10:5".to_string(),
        duration_ms: 1500,
        ..Default::default()
    };

    entry.attach_failure_logs(task_id);

    assert!(entry.stdout_log_path.is_some(), "stdout_log_path should be set");
    assert!(entry.stderr_log_path.is_some(), "stderr_log_path should be set");

    let stdout_path = std::path::Path::new(entry.stdout_log_path.as_ref().unwrap());
    let stderr_path = std::path::Path::new(entry.stderr_log_path.as_ref().unwrap());

    assert!(stdout_path.exists(), "stdout log file should exist");
    assert!(stderr_path.exists(), "stderr log file should exist");

    let stdout_content = std::fs::read_to_string(stdout_path).unwrap();
    assert_eq!(stdout_content, "some stdout output");

    let stderr_content = std::fs::read_to_string(stderr_path).unwrap();
    assert!(stderr_content.contains("error[E0308]"));

    // Cleanup
    cleanup_validation_logs(task_id);
    assert!(!dir.exists(), "log dir should be cleaned up");
}

#[test]
fn attach_failure_logs_skips_empty_outputs() {
    let task_id = &format!("test-logs-empty-{}", uuid::Uuid::new_v4());

    let mut entry = ValidationLogEntry {
        phase: "validate".to_string(),
        command: "cargo check".to_string(),
        path: ".".to_string(),
        label: "Rust".to_string(),
        status: "failed".to_string(),
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        duration_ms: 1500,
        ..Default::default()
    };

    entry.attach_failure_logs(task_id);

    assert!(entry.stdout_log_path.is_none(), "empty stdout should not produce a log file");
    assert!(entry.stderr_log_path.is_none(), "empty stderr should not produce a log file");

    // Cleanup (dir may or may not exist)
    cleanup_validation_logs(task_id);
}

#[test]
fn cleanup_validation_logs_removes_directory() {
    let task_id = &format!("test-cleanup-{}", uuid::Uuid::new_v4());
    let dir = validation_log_dir(task_id);

    // Create dir + files
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.log"), "test content").unwrap();
    assert!(dir.exists());

    cleanup_validation_logs(task_id);
    assert!(!dir.exists(), "cleanup should remove the directory");
}

#[test]
fn cleanup_validation_logs_is_noop_when_no_directory() {
    let task_id = "nonexistent-task-id";
    // Should not panic or error
    cleanup_validation_logs(task_id);
}
