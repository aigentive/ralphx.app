//! Rebase and fetch operations for the two-phase merge workflow
//!
//! Extracted from `merge.rs` — contains fetch, rebase, abort, continue,
//! and stale rebase completion operations.

use super::*;

impl GitService {
    // =========================================================================
    // Rebase Operations (Phase 1 - fast path)
    // =========================================================================

    /// Fetch from origin (if remote exists)
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub fn fetch_origin(repo: &Path) -> AppResult<()> {
        debug!("Fetching from origin in {:?}", repo);

        // Check if origin exists first
        let remote_check = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(repo)
            .output();

        if let Ok(output) = remote_check {
            if !output.status.success() {
                debug!("No origin remote configured, skipping fetch");
                return Ok(());
            }
        }

        let output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git fetch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git fetch failed (non-fatal): {}", stderr);
        }

        Ok(())
    }

    /// Rebase current branch onto a base branch
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    /// * `base` - Name of the base branch to rebase onto
    pub fn rebase_onto(path: &Path, base: &str) -> AppResult<RebaseResult> {
        debug!("Rebasing onto '{}' in {:?}", base, path);

        let output = Command::new("git")
            .args(["rebase", base])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git rebase: {}", e)))?;

        if output.status.success() {
            return Ok(RebaseResult::Success);
        }

        // Check if it's a conflict
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("CONFLICT") || stderr.contains("conflict") {
            let conflict_files = Self::get_conflict_files(path)?;

            // If no unmerged files and no conflict markers, git auto-resolved the conflicts
            // Try to continue the rebase programmatically
            if conflict_files.is_empty() && !Self::has_conflict_markers(path).unwrap_or(true) {
                debug!("Git auto-resolved conflicts, attempting to continue rebase");
                return Self::try_continue_rebase(path);
            }

            return Ok(RebaseResult::Conflict {
                files: conflict_files,
            });
        }

        Err(AppError::GitOperation(format!("Rebase failed: {}", stderr)))
    }

    /// Abort an in-progress rebase
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    pub fn abort_rebase(path: &Path) -> AppResult<()> {
        debug!("Aborting rebase in {:?}", path);

        let output = Command::new("git")
            .args(["rebase", "--abort"])
            .current_dir(path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rebase --abort: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't error if no rebase in progress
            if !stderr.contains("No rebase in progress") {
                return Err(AppError::GitOperation(format!(
                    "Failed to abort rebase: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    /// Continue an in-progress rebase, looping until completion or real conflicts
    ///
    /// When git auto-resolves all conflicts (stderr contains "CONFLICT" but no unmerged files),
    /// this helper programmatically completes the rebase by running `git add --all` +
    /// `git rebase --continue` (with GIT_EDITOR=true). Handles multi-step rebases by looping
    /// up to 50 iterations.
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    pub fn try_continue_rebase(path: &Path) -> AppResult<RebaseResult> {
        const MAX_ITERATIONS: u32 = 50;
        debug!("Attempting to continue rebase in {:?} (max {} iterations)", path, MAX_ITERATIONS);

        for iteration in 0..MAX_ITERATIONS {
            debug!("try_continue_rebase iteration {} of {}", iteration + 1, MAX_ITERATIONS);

            // Stage all changes
            let add_output = Command::new("git")
                .args(["add", "--all"])
                .current_dir(path)
                .output()
                .map_err(|e| AppError::GitOperation(format!("Failed to run git add: {}", e)))?;

            if !add_output.status.success() {
                let stderr = String::from_utf8_lossy(&add_output.stderr);
                return Err(AppError::GitOperation(format!("git add failed: {}", stderr)));
            }

            // Continue the rebase with GIT_EDITOR=true to auto-accept all prompts
            let continue_output = Command::new("git")
                .args(["rebase", "--continue"])
                .env("GIT_EDITOR", "true")
                .current_dir(path)
                .output()
                .map_err(|e| AppError::GitOperation(format!("Failed to run git rebase --continue: {}", e)))?;

            // Check if rebase completed successfully
            if continue_output.status.success() {
                debug!("Rebase completed successfully at iteration {}", iteration + 1);
                return Ok(RebaseResult::Success);
            }

            let stderr = String::from_utf8_lossy(&continue_output.stderr);
            let stdout = String::from_utf8_lossy(&continue_output.stdout);

            // Check if there are real conflicts (unmerged files or conflict markers)
            if stderr.contains("CONFLICT") || stderr.contains("conflict") || stdout.contains("CONFLICT") {
                let conflict_files = Self::get_conflict_files(path)?;
                if !conflict_files.is_empty() {
                    debug!("Real conflict detected at iteration {} with {} files", iteration + 1, conflict_files.len());
                    return Ok(RebaseResult::Conflict {
                        files: conflict_files,
                    });
                }

                // Check for conflict markers
                if Self::has_conflict_markers(path).unwrap_or(false) {
                    debug!("Conflict markers found at iteration {}", iteration + 1);
                    let conflict_files = Self::get_conflict_files(path)?;
                    return Ok(RebaseResult::Conflict {
                        files: conflict_files,
                    });
                }

                // No real conflicts detected, auto-resolved step - continue looping
                debug!("Auto-resolved step detected at iteration {}, continuing loop", iteration + 1);
                continue;
            }

            // Check for "No rebase in progress" - rebase already done
            if stderr.contains("No rebase in progress") || stdout.contains("No rebase in progress") {
                debug!("No rebase in progress - rebase completed successfully");
                return Ok(RebaseResult::Success);
            }

            // Unexpected error
            let error_msg = format!("Rebase --continue failed: stderr={} stdout={}", stderr, stdout);
            return Err(AppError::GitOperation(error_msg));
        }

        // Hit iteration limit without completing
        Err(AppError::GitOperation(format!(
            "Rebase did not complete within {} iterations",
            MAX_ITERATIONS
        )))
    }

    /// Attempt to complete a stale rebase by checking if conflicts are resolved
    ///
    /// Called during merge timeout recovery. If a rebase is in progress:
    /// - Checks if all conflicts are resolved (no unmerged files, no conflict markers)
    /// - If resolved: runs `git add --all` + `git rebase --continue` (with GIT_EDITOR=true)
    /// - Returns Completed if the rebase finishes, HasConflicts if real conflicts remain
    /// - Loops up to 50 iterations for multi-step rebases
    ///
    /// # Arguments
    /// * `worktree` - Path to the git worktree or repository
    pub fn try_complete_stale_rebase(worktree: &Path) -> StaleRebaseResult {
        const MAX_ITERATIONS: u32 = 50;

        // Check if a rebase is actually in progress
        if !Self::is_rebase_in_progress(worktree) {
            return StaleRebaseResult::NoRebase;
        }

        debug!("Attempting to complete stale rebase in {:?}", worktree);

        for iteration in 0..MAX_ITERATIONS {
            // Check for unmerged files
            match Self::get_conflict_files(worktree) {
                Ok(conflict_files) if !conflict_files.is_empty() => {
                    debug!(
                        "Real conflicts detected in stale rebase iteration {} with {} files",
                        iteration + 1,
                        conflict_files.len()
                    );
                    return StaleRebaseResult::HasConflicts {
                        files: conflict_files,
                    };
                }
                Err(e) => {
                    return StaleRebaseResult::Failed {
                        reason: format!("Failed to check conflict files: {}", e),
                    };
                }
                _ => {}
            }

            // Check for conflict markers
            if Self::has_conflict_markers(worktree).unwrap_or(false) {
                debug!("Conflict markers found in stale rebase iteration {}", iteration + 1);
                match Self::get_conflict_files(worktree) {
                    Ok(files) => {
                        return StaleRebaseResult::HasConflicts { files };
                    }
                    Err(e) => {
                        return StaleRebaseResult::Failed {
                            reason: format!("Failed to get conflict files after marker check: {}", e),
                        };
                    }
                }
            }

            // No real conflicts - attempt to continue
            debug!("No real conflicts in iteration {}, attempting to continue rebase", iteration + 1);

            // Stage all changes
            let add_output = Command::new("git")
                .args(["add", "--all"])
                .current_dir(worktree)
                .output();

            let add_status = match add_output {
                Ok(output) if output.status.success() => true,
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    debug!("git add failed: {}", stderr);
                    return StaleRebaseResult::Failed {
                        reason: format!("git add failed: {}", stderr),
                    };
                }
                Err(e) => {
                    return StaleRebaseResult::Failed {
                        reason: format!("Failed to run git add: {}", e),
                    };
                }
            };

            if !add_status {
                return StaleRebaseResult::Failed {
                    reason: "git add failed".to_string(),
                };
            }

            // Continue the rebase
            let continue_output = Command::new("git")
                .args(["rebase", "--continue"])
                .env("GIT_EDITOR", "true")
                .current_dir(worktree)
                .output();

            let (continue_status, stdout, stderr) = match continue_output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    (output.status.success(), stdout, stderr)
                }
                Err(e) => {
                    return StaleRebaseResult::Failed {
                        reason: format!("Failed to run git rebase --continue: {}", e),
                    };
                }
            };

            // Check if rebase completed
            if continue_status {
                debug!("Stale rebase completed successfully at iteration {}", iteration + 1);
                return StaleRebaseResult::Completed;
            }

            // Check if no rebase in progress (already done)
            if stderr.contains("No rebase in progress") || stdout.contains("No rebase in progress") {
                debug!("No rebase in progress - rebase already completed");
                return StaleRebaseResult::Completed;
            }

            // Check for conflict
            if stderr.contains("CONFLICT") || stderr.contains("conflict") || stdout.contains("CONFLICT") {
                // Re-check for real conflicts
                match Self::get_conflict_files(worktree) {
                    Ok(conflict_files) if !conflict_files.is_empty() => {
                        debug!(
                            "Real conflicts found after rebase --continue in iteration {}",
                            iteration + 1
                        );
                        return StaleRebaseResult::HasConflicts {
                            files: conflict_files,
                        };
                    }
                    Ok(_) => {
                        // Auto-resolved step, continue looping
                        debug!("Auto-resolved step at iteration {}, continuing", iteration + 1);
                        continue;
                    }
                    Err(e) => {
                        return StaleRebaseResult::Failed {
                            reason: format!(
                                "Failed to check conflicts after --continue: {}",
                                e
                            ),
                        };
                    }
                }
            } else {
                // Unexpected error from git rebase --continue
                return StaleRebaseResult::Failed {
                    reason: format!(
                        "Unexpected git rebase --continue failure: stderr={} stdout={}",
                        stderr, stdout
                    ),
                };
            }
        }

        // Hit iteration limit
        StaleRebaseResult::Failed {
            reason: format!(
                "Rebase did not complete within {} iterations",
                MAX_ITERATIONS
            ),
        }
    }
}
