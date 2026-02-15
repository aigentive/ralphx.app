// Merge validation: post-merge validation gate
//
// Extracted from side_effects.rs — runs project analysis commands to verify merge correctness.
// Decomposed into setup phase, validate phase, and orchestrator.

use std::path::Path;
use std::process::Command;

use tauri::{AppHandle, Emitter};

use crate::domain::entities::{
    merge_progress_event::{
        map_command_to_phase, MergePhase, MergePhaseStatus, MergeProgressEvent,
    },
    Project, Task,
};

use super::merge_helpers::truncate_str;

/// Analysis entry for path-scoped build/validation commands.
/// Mirrors the HTTP handler's AnalysisEntry but kept local to avoid cross-module coupling.
#[derive(Debug, Clone, serde::Deserialize)]
struct MergeAnalysisEntry {
    path: String,
    #[allow(dead_code)]
    label: String,
    #[serde(default)]
    validate: Vec<String>,
    #[serde(default)]
    worktree_setup: Vec<String>,
}

/// Analysis entry for pre-execution setup commands.
/// Includes the `install` field (unlike MergeAnalysisEntry which omits it).
#[derive(Debug, Clone, serde::Deserialize)]
struct PreExecAnalysisEntry {
    path: String,
    label: String,
    #[serde(default)]
    install: Option<String>,
    #[serde(default)]
    worktree_setup: Vec<String>,
}

/// A single validation command execution record for streaming + storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ValidationLogEntry {
    pub(super) phase: String,
    pub(super) command: String,
    pub(super) path: String,
    pub(super) label: String,
    pub(super) status: String,
    pub(super) exit_code: Option<i32>,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) duration_ms: u64,
}

/// Result of running post-merge validation commands.
#[derive(Debug)]
pub(crate) struct ValidationResult {
    pub(crate) all_passed: bool,
    pub(crate) failures: Vec<ValidationFailure>,
    pub(crate) log: Vec<ValidationLogEntry>,
}

#[derive(Debug)]
pub(crate) struct ValidationFailure {
    pub(super) command: String,
    pub(super) path: String,
    pub(super) exit_code: Option<i32>,
    pub(super) stderr: String,
}

/// Result of running pre-execution setup commands.
#[derive(Debug)]
pub(crate) struct PreExecSetupResult {
    pub(crate) success: bool,
    pub(crate) log: Vec<ValidationLogEntry>,
}

/// Truncate a string to `max_len` chars, appending "... (truncated)" if needed.
fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... (truncated)", &s[..max_len])
    }
}

/// Emit a high-level merge progress event for UI display
pub(super) fn emit_merge_progress<R: tauri::Runtime>(
    app_handle: Option<&AppHandle<R>>,
    task_id: &str,
    phase: MergePhase,
    status: MergePhaseStatus,
    message: String,
) {
    if let Some(handle) = app_handle {
        let event = MergeProgressEvent::new(task_id.to_string(), phase, status, message);
        let _ = handle.emit("task:merge_progress", event);
    }
}

/// Run worktree setup commands (symlinks, etc.) — non-fatal.
///
/// Returns the log entries and whether any setup command failed.
async fn run_setup_phase(
    entries: &[MergeAnalysisEntry],
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    context: Option<&str>,
) -> (Vec<ValidationLogEntry>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut setup_had_failures = false;

    let has_setup_commands = entries.iter().any(|e| !e.worktree_setup.is_empty());
    if has_setup_commands {
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::WorktreeSetup,
            MergePhaseStatus::Started,
            "Setting up worktree environment".to_string(),
        );
    }

    let setup_phase_start = std::time::Instant::now();
    let mut setup_count = 0;
    for entry in entries {
        setup_count += entry.worktree_setup.len();
    }
    if setup_count > 0 {
        tracing::info!(
            task_id = task_id_str,
            command_count = setup_count,
            "run_validation_commands: starting setup phase"
        );
    }

    for entry in entries {
        for cmd_str in &entry.worktree_setup {
            let resolved_cmd = resolve(cmd_str);
            let resolved_path = resolve(&entry.path);
            let cmd_cwd = if resolved_path == "." {
                merge_cwd.to_path_buf()
            } else {
                merge_cwd.join(&resolved_path)
            };

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let mut event_data = serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "setup",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                });
                if let Some(ctx) = context {
                    event_data["context"] = serde_json::json!(ctx);
                }
                let _ = handle.emit("merge:validation_step", event_data);
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running worktree setup command"
            );

            let start = std::time::Instant::now();

            // Clone for move into spawn_blocking
            let resolved_cmd_clone = resolved_cmd.clone();
            let cmd_cwd_clone = cmd_cwd.clone();

            let result = tokio::task::spawn_blocking(move || {
                Command::new("sh")
                    .arg("-c")
                    .arg(&resolved_cmd_clone)
                    .current_dir(&cmd_cwd_clone)
                    .output()
            })
            .await;

            let log_entry = match result {
                Ok(Ok(output)) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let status = if output.status.success() {
                        "success"
                    } else {
                        "failed"
                    };

                    if !output.status.success() {
                        setup_had_failures = true;
                        tracing::warn!(
                            command = %resolved_cmd,
                            stderr = %stderr_raw,
                            "Worktree setup command failed (non-fatal)"
                        );
                    }

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                    }
                }
                Ok(Err(e)) => {
                    setup_had_failures = true;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::warn!(
                        command = %resolved_cmd,
                        error = %e,
                        "Worktree setup command failed (non-fatal)"
                    );

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                    }
                }
                Err(e) => {
                    setup_had_failures = true;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(
                        command = %resolved_cmd,
                        error = %e,
                        "Worktree setup task panicked or was cancelled"
                    );

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Task failed: {}", e), 2000),
                        duration_ms,
                    }
                }
            };

            if let Some(handle) = app_handle {
                let mut event_data = serde_json::json!({
                    "task_id": task_id_str,
                    "phase": log_entry.phase,
                    "command": log_entry.command,
                    "path": log_entry.path,
                    "label": log_entry.label,
                    "status": log_entry.status,
                    "exit_code": log_entry.exit_code,
                    "stdout": log_entry.stdout,
                    "stderr": log_entry.stderr,
                    "duration_ms": log_entry.duration_ms,
                });
                if let Some(ctx) = context {
                    event_data["context"] = serde_json::json!(ctx);
                }
                let _ = handle.emit("merge:validation_step", event_data);
            }
            log.push(log_entry);
        }
    }

    if setup_count > 0 {
        let setup_duration_ms = setup_phase_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = setup_duration_ms,
            command_count = setup_count,
            "run_validation_commands: completed setup phase"
        );
    }

    // Emit worktree setup completion event
    if has_setup_commands {
        if setup_had_failures {
            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::WorktreeSetup,
                MergePhaseStatus::Failed,
                "Worktree setup completed with warnings (non-fatal)".to_string(),
            );
        } else {
            emit_merge_progress(
                app_handle,
                task_id_str,
                MergePhase::WorktreeSetup,
                MergePhaseStatus::Passed,
                "Worktree setup completed successfully".to_string(),
            );
        }
    }

    (log, setup_had_failures)
}

/// Run validate-phase commands, checking cache where possible.
///
/// Returns (log_entries, failures, ran_any).
async fn run_validate_phase(
    entries: &[MergeAnalysisEntry],
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    cached_log: Option<&[ValidationLogEntry]>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
) -> (Vec<ValidationLogEntry>, Vec<ValidationFailure>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut failures = Vec::new();
    let mut ran_any = false;

    let validate_count: usize = entries.iter().map(|e| e.validate.len()).sum();
    let validate_phase_start = std::time::Instant::now();
    if validate_count > 0 {
        tracing::info!(
            task_id = task_id_str,
            command_count = validate_count,
            "run_validation_commands: starting validate phase"
        );
    }

    for entry in entries {
        if entry.validate.is_empty() {
            continue;
        }

        let resolved_path = resolve(&entry.path);
        let cmd_cwd = if resolved_path == "." {
            merge_cwd.to_path_buf()
        } else {
            merge_cwd.join(&resolved_path)
        };

        for cmd_str in &entry.validate {
            let resolved_cmd = resolve(cmd_str);
            ran_any = true;

            // Check cache: skip previously-passed validate commands when SHA matches
            if let Some(cached) = cached_log {
                let cached_hit = cached.iter().find(|c| {
                    c.phase == "validate"
                        && c.command == resolved_cmd
                        && c.path == resolved_path
                        && (c.status == "success" || c.status == "cached")
                });
                if let Some(prev) = cached_hit {
                    tracing::info!(
                        command = %resolved_cmd,
                        "Skipping validation command (cached, SHA unchanged)"
                    );

                    let phase = map_command_to_phase(&resolved_cmd);
                    emit_merge_progress(
                        app_handle,
                        task_id_str,
                        phase,
                        MergePhaseStatus::Passed,
                        format!("{} (cached)", resolved_cmd),
                    );

                    let log_entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "cached".to_string(),
                        exit_code: prev.exit_code,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                    };
                    if let Some(handle) = app_handle {
                        let _ = handle.emit(
                            "merge:validation_step",
                            serde_json::json!({
                                "task_id": task_id_str,
                                "phase": log_entry.phase,
                                "command": log_entry.command,
                                "path": log_entry.path,
                                "label": log_entry.label,
                                "status": log_entry.status,
                                "exit_code": log_entry.exit_code,
                                "duration_ms": log_entry.duration_ms,
                            }),
                        );
                    }
                    log.push(log_entry);
                    continue;
                }
            }

            // Emit high-level merge progress event
            let phase = map_command_to_phase(&resolved_cmd);
            emit_merge_progress(
                app_handle,
                task_id_str,
                phase,
                MergePhaseStatus::Started,
                format!("Running {}", resolved_cmd),
            );

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": "validate",
                        "command": resolved_cmd,
                        "path": resolved_path,
                        "label": entry.label,
                        "status": "running",
                    }),
                );
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running post-merge validation command"
            );

            let start = std::time::Instant::now();

            // Clone for move into spawn_blocking
            let resolved_cmd_clone = resolved_cmd.clone();
            let cmd_cwd_clone = cmd_cwd.clone();

            let result = tokio::task::spawn_blocking(move || {
                Command::new("sh")
                    .arg("-c")
                    .arg(&resolved_cmd_clone)
                    .current_dir(&cmd_cwd_clone)
                    .output()
            })
            .await;

            let (log_entry, failure) = match result {
                Ok(Ok(output)) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();

                    let failure = if !output.status.success() {
                        tracing::warn!(
                            command = %resolved_cmd,
                            exit_code = ?output.status.code(),
                            stderr = %stderr_raw,
                            "Post-merge validation command failed"
                        );
                        Some(ValidationFailure {
                            command: resolved_cmd.clone(),
                            path: resolved_path.clone(),
                            exit_code: output.status.code(),
                            stderr: format!(
                                "{}{}",
                                if stderr_raw.is_empty() {
                                    ""
                                } else {
                                    &stderr_raw
                                },
                                if stdout_raw.is_empty() {
                                    String::new()
                                } else {
                                    format!("\nstdout: {}", stdout_raw)
                                },
                            ),
                        })
                    } else {
                        tracing::info!(command = %resolved_cmd, "Post-merge validation command passed");
                        None
                    };

                    // Emit high-level merge progress completion event
                    if output.status.success() {
                        emit_merge_progress(
                            app_handle,
                            task_id_str,
                            phase,
                            MergePhaseStatus::Passed,
                            format!("{} passed", resolved_cmd),
                        );
                    } else {
                        emit_merge_progress(
                            app_handle,
                            task_id_str,
                            phase,
                            MergePhaseStatus::Failed,
                            format!("{} failed", resolved_cmd),
                        );
                    }

                    let status = if output.status.success() {
                        "success"
                    } else {
                        "failed"
                    };
                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                    };

                    (entry, failure)
                }
                Ok(Err(e)) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(command = %resolved_cmd, error = %e, "Failed to execute validation command");

                    emit_merge_progress(
                        app_handle,
                        task_id_str,
                        phase,
                        MergePhaseStatus::Failed,
                        format!("Failed to execute: {}", e),
                    );

                    let failure = ValidationFailure {
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        exit_code: None,
                        stderr: format!("Failed to execute: {}", e),
                    };

                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                    };

                    (entry, Some(failure))
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(
                        command = %resolved_cmd,
                        error = %e,
                        "Validation task panicked or was cancelled"
                    );

                    let failure = ValidationFailure {
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        exit_code: None,
                        stderr: format!("Task failed: {}", e),
                    };

                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "failed".to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Task failed: {}", e), 2000),
                        duration_ms,
                    };

                    (entry, Some(failure))
                }
            };

            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": log_entry.phase,
                        "command": log_entry.command,
                        "path": log_entry.path,
                        "label": log_entry.label,
                        "status": log_entry.status,
                        "exit_code": log_entry.exit_code,
                        "stdout": log_entry.stdout,
                        "stderr": log_entry.stderr,
                        "duration_ms": log_entry.duration_ms,
                    }),
                );
            }
            log.push(log_entry);
            if let Some(f) = failure {
                failures.push(f);
            }
        }
    }

    if validate_count > 0 {
        let validate_duration_ms = validate_phase_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = validate_duration_ms,
            command_count = validate_count,
            failure_count = failures.len(),
            "run_validation_commands: completed validate phase"
        );
    }

    (log, failures, ran_any)
}

/// Load effective analysis, resolve template vars, and run all validate commands.
///
/// Returns `None` if no analysis entries exist (backward compatible — skip validation).
/// Returns `Some(ValidationResult)` with pass/fail details otherwise.
///
/// When `app_handle` is `Some`, emits `merge:validation_step` events for real-time UI streaming.
/// All executed commands are recorded in `ValidationResult::log` for metadata storage.
///
/// When `cached_log` is provided, validate-phase commands that previously passed (status
/// "success" or "cached") are skipped and emitted as "cached" instead of re-running.
/// Setup-phase commands always re-run. Previously-failed commands always re-run.
pub(crate) async fn run_validation_commands(
    project: &Project,
    task: &Task,
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    cached_log: Option<&[ValidationLogEntry]>,
) -> Option<ValidationResult> {
    let overall_start = std::time::Instant::now();
    tracing::info!(
        task_id = task_id_str,
        cwd = %merge_cwd.display(),
        "run_validation_commands: starting validation"
    );

    // Load effective analysis: custom_analysis ?? detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref())?;

    let entries: Vec<MergeAnalysisEntry> = match serde_json::from_str(analysis_json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse project analysis JSON, skipping validation");
            return None;
        }
    };

    if entries.is_empty() {
        return None;
    }

    // Build template resolver
    let project_root = &project.working_directory;
    let worktree_path = merge_cwd.to_str().unwrap_or(project_root);
    let task_branch = task.task_branch.as_deref().unwrap_or("");

    let project_root_owned = project_root.clone();
    let worktree_path_owned = worktree_path.to_string();
    let task_branch_owned = task_branch.to_string();

    let resolve = |s: &str| -> String {
        s.replace("{project_root}", &project_root_owned)
            .replace("{worktree_path}", &worktree_path_owned)
            .replace("{task_branch}", &task_branch_owned)
    };

    // Phase 1: Setup
    let (mut log, _setup_had_failures) =
        run_setup_phase(&entries, merge_cwd, task_id_str, app_handle, &resolve, None).await;

    // Phase 2: Validate
    let (validate_log, failures, ran_any) = run_validate_phase(
        &entries,
        merge_cwd,
        task_id_str,
        app_handle,
        cached_log,
        &resolve,
    )
    .await;

    log.extend(validate_log);

    if !ran_any {
        let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = overall_duration_ms,
            "run_validation_commands: no validation commands to run"
        );
        return None;
    }

    let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
    let all_passed = failures.is_empty();
    tracing::info!(
        task_id = task_id_str,
        duration_ms = overall_duration_ms,
        all_passed = all_passed,
        failure_count = failures.len(),
        total_commands = log.len(),
        "run_validation_commands: completed validation"
    );

    Some(ValidationResult {
        all_passed,
        failures,
        log,
    })
}

/// Format validation failures as a JSON metadata string for MergeIncomplete.
pub(crate) fn format_validation_error_metadata(
    failures: &[ValidationFailure],
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    let failure_details: Vec<serde_json::Value> = failures
        .iter()
        .map(|f| {
            serde_json::json!({
                "command": f.command,
                "path": f.path,
                "exit_code": f.exit_code,
                "stderr": truncate_str(&f.stderr, 2000),
            })
        })
        .collect();

    serde_json::json!({
        "error": format!("Merge validation failed: {} command(s) failed", failures.len()),
        "validation_failures": failure_details,
        "validation_log": log,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Check if task metadata has the skip_validation flag set, and clear it (one-shot).
pub(super) fn take_skip_validation_flag(task: &mut Task) -> bool {
    let Some(meta_str) = task.metadata.as_ref() else {
        return false;
    };
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(meta_str) else {
        return false;
    };
    let flag = val
        .get("skip_validation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if flag {
        if let Some(obj) = val.as_object_mut() {
            obj.remove("skip_validation");
            task.metadata = Some(val.to_string());
        }
    }
    flag
}

/// Format validation warnings as a JSON metadata string for Warn mode.
/// Stores the log but allows merge to proceed.
pub(super) fn format_validation_warn_metadata(
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    serde_json::json!({
        "validation_log": log,
        "validation_warnings": true,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Extract cached validation log from task metadata if the source branch SHA matches.
///
/// Returns `Some(entries)` when the previous validation ran against the same source SHA,
/// meaning the branch code has not changed and previously-passed checks can be skipped.
///
/// Note: Caching is effective in worktree mode. In local mode, rebase rewrites the source
/// branch SHA on each retry, so cache hits are rare.
pub(super) fn extract_cached_validation(
    task: &Task,
    current_sha: &str,
) -> Option<Vec<ValidationLogEntry>> {
    let meta_str = task.metadata.as_ref()?;
    let val: serde_json::Value = serde_json::from_str(meta_str).ok()?;
    let stored_sha = val.get("validation_source_sha")?.as_str()?;
    if stored_sha != current_sha {
        return None;
    }
    let log_val = val.get("validation_log")?;
    serde_json::from_value::<Vec<ValidationLogEntry>>(log_val.clone()).ok()
}

/// Run install commands for pre-execution setup.
/// Returns (log_entries, had_failures).
async fn run_install_phase(
    entries: &[PreExecAnalysisEntry],
    exec_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    context: &str,
) -> (Vec<ValidationLogEntry>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut install_had_failures = false;

    for entry in entries {
        let Some(ref cmd_str) = entry.install else {
            continue;
        };

        let resolved_cmd = resolve(cmd_str);
        let resolved_path = resolve(&entry.path);
        let cmd_cwd = if resolved_path == "." {
            exec_cwd.to_path_buf()
        } else {
            exec_cwd.join(&resolved_path)
        };

        // Emit "running" event before execution
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "merge:validation_step",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "install",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                    "context": context,
                }),
            );
        }

        tracing::info!(
            command = %resolved_cmd,
            cwd = %cmd_cwd.display(),
            "Running pre-execution install command"
        );

        let start = std::time::Instant::now();

        // Clone for move into spawn_blocking
        let resolved_cmd_clone = resolved_cmd.clone();
        let cmd_cwd_clone = cmd_cwd.clone();

        let result = tokio::task::spawn_blocking(move || {
            Command::new("sh")
                .arg("-c")
                .arg(&resolved_cmd_clone)
                .current_dir(&cmd_cwd_clone)
                .output()
        })
        .await;

        let log_entry = match result {
            Ok(Ok(output)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                let status = if output.status.success() {
                    "success"
                } else {
                    "failed"
                };

                if !output.status.success() {
                    install_had_failures = true;
                    tracing::warn!(
                        command = %resolved_cmd,
                        stderr = %stderr_raw,
                        "Pre-execution install command failed"
                    );
                }

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: status.to_string(),
                    exit_code: output.status.code(),
                    stdout: truncate_output(&stdout_raw, 2000),
                    stderr: truncate_output(&stderr_raw, 2000),
                    duration_ms,
                }
            }
            Ok(Err(e)) => {
                install_had_failures = true;
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::warn!(
                    command = %resolved_cmd,
                    error = %e,
                    "Pre-execution install command failed"
                );

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: "failed".to_string(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                    duration_ms,
                }
            }
            Err(e) => {
                install_had_failures = true;
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(
                    command = %resolved_cmd,
                    error = %e,
                    "Pre-execution install task panicked or was cancelled"
                );

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: "failed".to_string(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: truncate_output(&format!("Task failed: {}", e), 2000),
                    duration_ms,
                }
            }
        };

        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "merge:validation_step",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phase": log_entry.phase,
                    "command": log_entry.command,
                    "path": log_entry.path,
                    "label": log_entry.label,
                    "status": log_entry.status,
                    "exit_code": log_entry.exit_code,
                    "stdout": log_entry.stdout,
                    "stderr": log_entry.stderr,
                    "duration_ms": log_entry.duration_ms,
                    "context": context,
                }),
            );
        }
        log.push(log_entry);
    }

    (log, install_had_failures)
}

/// Load effective analysis, resolve template vars, and run pre-execution setup commands.
///
/// Returns `None` if no analysis entries exist (backward compatible — skip setup).
/// Returns `Some(PreExecSetupResult)` with success/failure details otherwise.
///
/// When `app_handle` is `Some`, emits `merge:validation_step` events with the provided context
/// for real-time UI streaming. All executed commands are recorded in `PreExecSetupResult::log`
/// for metadata storage.
///
/// This function runs worktree_setup + install commands only. No validate steps.
pub(crate) async fn run_pre_execution_setup(
    project: &Project,
    task: &Task,
    exec_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    context: &str,
) -> Option<PreExecSetupResult> {
    let overall_start = std::time::Instant::now();
    tracing::info!(
        task_id = task_id_str,
        cwd = %exec_cwd.display(),
        "run_pre_execution_setup: starting pre-execution setup"
    );

    // Load effective analysis: custom_analysis ?? detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref())?;

    let entries: Vec<PreExecAnalysisEntry> = match serde_json::from_str(analysis_json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse project analysis JSON, skipping pre-execution setup");
            return None;
        }
    };

    if entries.is_empty() {
        return None;
    }

    // Build template resolver
    let project_root = &project.working_directory;
    let worktree_path = exec_cwd.to_str().unwrap_or(project_root);
    let task_branch = task.task_branch.as_deref().unwrap_or("");

    let project_root_owned = project_root.clone();
    let worktree_path_owned = worktree_path.to_string();
    let task_branch_owned = task_branch.to_string();

    let resolve = |s: &str| -> String {
        s.replace("{project_root}", &project_root_owned)
            .replace("{worktree_path}", &worktree_path_owned)
            .replace("{task_branch}", &task_branch_owned)
    };

    // Phase 1: Worktree Setup (non-fatal, reuse existing run_setup_phase logic)
    // Convert PreExecAnalysisEntry to MergeAnalysisEntry for reuse
    let merge_entries: Vec<MergeAnalysisEntry> = entries
        .iter()
        .map(|e| MergeAnalysisEntry {
            path: e.path.clone(),
            label: e.label.clone(),
            validate: Vec::new(),
            worktree_setup: e.worktree_setup.clone(),
        })
        .collect();

    let (mut log, _setup_had_failures) = run_setup_phase(
        &merge_entries,
        exec_cwd,
        task_id_str,
        app_handle,
        &resolve,
        Some(context),
    )
    .await;

    // Phase 2: Install (fatal based on merge_validation_mode)
    let (install_log, install_had_failures) = run_install_phase(
        &entries,
        exec_cwd,
        task_id_str,
        app_handle,
        &resolve,
        context,
    )
    .await;

    log.extend(install_log);

    let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
    let success = !install_had_failures;
    tracing::info!(
        task_id = task_id_str,
        duration_ms = overall_duration_ms,
        success = success,
        total_commands = log.len(),
        "run_pre_execution_setup: completed pre-execution setup"
    );

    Some(PreExecSetupResult { success, log })
}
