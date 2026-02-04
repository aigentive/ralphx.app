//! Diff Service - Extracts file changes from agent activity and git
//!
//! Provides file change information for the DiffViewer by:
//! 1. Querying activity events to find Write/Edit tool calls
//! 2. Using git to get actual diff content

use crate::domain::entities::TaskId;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// A file that was changed by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// File path relative to project root
    pub path: String,
    /// Change status
    pub status: FileChangeStatus,
    /// Number of lines added
    pub additions: u32,
    /// Number of lines deleted
    pub deletions: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
}

/// Diff data for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// File path
    pub file_path: String,
    /// Content before changes (empty for new files)
    pub old_content: String,
    /// Content after changes (empty for deleted files)
    pub new_content: String,
    /// Programming language for syntax highlighting
    pub language: String,
}

/// Service for extracting diff information
#[derive(Default)]
pub struct DiffService;

impl DiffService {
    pub fn new() -> Self {
        Self
    }

    /// Get all files changed by the agent for a task
    /// Compares against base_branch to show all changes since branching
    /// Uses git diff directly instead of activity events to capture all changes
    pub async fn get_task_file_changes(
        &self,
        _task_id: &TaskId,
        project_path: &str,
        base_branch: &str,
    ) -> AppResult<Vec<FileChange>> {
        self.get_file_changes_between_refs(project_path, base_branch, "HEAD")
    }

    /// Get line additions/deletions for a file compared to base branch
    #[allow(dead_code)]
    fn get_file_line_counts(
        &self,
        file_path: &str,
        project_path: &str,
        base_branch: &str,
    ) -> (u32, u32) {
        let output = Command::new("git")
            .args(["diff", "--numstat", base_branch, "--", file_path])
            .current_dir(project_path)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let additions: u32 = parts[0].parse().unwrap_or(0);
                    let deletions: u32 = parts[1].parse().unwrap_or(0);
                    return (additions, deletions);
                }
            }
        }

        (0, 0)
    }

    /// Get the diff content for a specific file
    /// Shows old content from base_branch for accurate comparison
    pub fn get_file_diff(
        &self,
        file_path: &str,
        project_path: &str,
        base_branch: &str,
    ) -> AppResult<FileDiff> {
        let full_path = std::path::Path::new(project_path).join(file_path);
        let new_content = std::fs::read_to_string(&full_path).unwrap_or_default();
        self.get_file_diff_between_refs_with_new(
            file_path,
            project_path,
            base_branch,
            &new_content,
        )
    }

    /// Get files changed in a specific commit
    pub fn get_commit_file_changes(
        &self,
        commit_sha: &str,
        project_path: &str,
    ) -> AppResult<Vec<FileChange>> {
        // Use git diff-tree to get files changed in this commit
        // Format: status\tpath (e.g., "A\tfile.rs" for added, "M\tfile.rs" for modified)
        let output = Command::new("git")
            .args([
                "diff-tree",
                "--no-commit-id",
                "--name-status",
                "-r",
                commit_sha,
            ])
            .current_dir(project_path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git diff-tree: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "git diff-tree failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut changes = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let status_char = parts[0].chars().next().unwrap_or('M');
                let file_path = parts[1];

                let status = match status_char {
                    'A' => FileChangeStatus::Added,
                    'D' => FileChangeStatus::Deleted,
                    _ => FileChangeStatus::Modified, // M, R, C, etc.
                };

                // Get line counts using git diff for this specific commit
                let (additions, deletions) =
                    self.get_commit_file_line_counts(commit_sha, file_path, project_path);

                changes.push(FileChange {
                    path: file_path.to_string(),
                    status,
                    additions,
                    deletions,
                });
            }
        }

        // Sort by path for consistent ordering
        changes.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(changes)
    }

    /// Get line additions/deletions for a file in a specific commit
    fn get_commit_file_line_counts(
        &self,
        commit_sha: &str,
        file_path: &str,
        project_path: &str,
    ) -> (u32, u32) {
        // git diff commit^..commit --numstat -- file_path
        let output = Command::new("git")
            .args([
                "diff",
                "--numstat",
                &format!("{}^", commit_sha),
                commit_sha,
                "--",
                file_path,
            ])
            .current_dir(project_path)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let additions: u32 = parts[0].parse().unwrap_or(0);
                    let deletions: u32 = parts[1].parse().unwrap_or(0);
                    return (additions, deletions);
                }
            }
        }

        (0, 0)
    }

    /// Get diff for a file in a specific commit (comparing to its parent)
    pub fn get_commit_file_diff(
        &self,
        commit_sha: &str,
        file_path: &str,
        project_path: &str,
    ) -> AppResult<FileDiff> {
        self.get_file_diff_between_refs(
            file_path,
            project_path,
            &format!("{}^", commit_sha),
            commit_sha,
        )
    }

    /// Get file changes between two refs (used for merged tasks and range diffs)
    pub fn get_file_changes_between_refs(
        &self,
        project_path: &str,
        from_ref: &str,
        to_ref: &str,
    ) -> AppResult<Vec<FileChange>> {
        let output = Command::new("git")
            .args(["diff", "--name-status", from_ref, to_ref])
            .current_dir(project_path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git diff: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "git diff failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut changes = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let status_char = parts[0].chars().next().unwrap_or('M');
                let file_path = parts[1];

                let status = match status_char {
                    'A' => FileChangeStatus::Added,
                    'D' => FileChangeStatus::Deleted,
                    _ => FileChangeStatus::Modified, // M, R, C, etc.
                };

                let (additions, deletions) = self.get_file_line_counts_between_refs(
                    file_path,
                    project_path,
                    from_ref,
                    to_ref,
                );

                changes.push(FileChange {
                    path: file_path.to_string(),
                    status,
                    additions,
                    deletions,
                });
            }
        }

        changes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(changes)
    }

    /// Get line additions/deletions for a file between two refs
    fn get_file_line_counts_between_refs(
        &self,
        file_path: &str,
        project_path: &str,
        from_ref: &str,
        to_ref: &str,
    ) -> (u32, u32) {
        let output = Command::new("git")
            .args(["diff", "--numstat", from_ref, to_ref, "--", file_path])
            .current_dir(project_path)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let additions: u32 = parts[0].parse().unwrap_or(0);
                    let deletions: u32 = parts[1].parse().unwrap_or(0);
                    return (additions, deletions);
                }
            }
        }

        (0, 0)
    }

    /// Get diff for a file between two refs
    pub fn get_file_diff_between_refs(
        &self,
        file_path: &str,
        project_path: &str,
        from_ref: &str,
        to_ref: &str,
    ) -> AppResult<FileDiff> {
        let old_content = self
            .get_file_content_at_ref(project_path, from_ref, file_path)
            .unwrap_or_default();
        let new_content = self
            .get_file_content_at_ref(project_path, to_ref, file_path)
            .unwrap_or_default();

        let language = get_language_from_path(file_path);
        Ok(FileDiff {
            file_path: file_path.to_string(),
            old_content,
            new_content,
            language,
        })
    }

    /// Same as get_file_diff_between_refs, but with new content already read from disk.
    fn get_file_diff_between_refs_with_new(
        &self,
        file_path: &str,
        project_path: &str,
        from_ref: &str,
        new_content: &str,
    ) -> AppResult<FileDiff> {
        let old_content = self
            .get_file_content_at_ref(project_path, from_ref, file_path)
            .unwrap_or_default();
        let language = get_language_from_path(file_path);
        Ok(FileDiff {
            file_path: file_path.to_string(),
            old_content,
            new_content: new_content.to_string(),
            language,
        })
    }

    /// Determine if a commit has a second parent (true merge commit)
    pub fn is_merge_commit(&self, project_path: &str, commit_sha: &str) -> bool {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("{}^2", commit_sha)])
            .current_dir(project_path)
            .output();
        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Compute base ref for a merged task range.
    /// If merge commit, use first parent; otherwise use merge-base with base branch.
    pub fn get_merged_base_ref(
        &self,
        project_path: &str,
        base_branch: &str,
        merge_commit_sha: &str,
    ) -> String {
        if self.is_merge_commit(project_path, merge_commit_sha) {
            return format!("{}^1", merge_commit_sha);
        }

        let output = Command::new("git")
            .args(["merge-base", base_branch, merge_commit_sha])
            .current_dir(project_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let base = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !base.is_empty() {
                    return base;
                }
            }
        }

        base_branch.to_string()
    }

    /// Get file changes for a merged task using merge commit range.
    pub fn get_merged_task_file_changes(
        &self,
        project_path: &str,
        base_branch: &str,
        merge_commit_sha: &str,
    ) -> AppResult<Vec<FileChange>> {
        let from_ref = self.get_merged_base_ref(project_path, base_branch, merge_commit_sha);
        self.get_file_changes_between_refs(project_path, &from_ref, merge_commit_sha)
    }

    /// Get file diff for a merged task using merge commit range.
    pub fn get_merged_task_file_diff(
        &self,
        file_path: &str,
        project_path: &str,
        base_branch: &str,
        merge_commit_sha: &str,
    ) -> AppResult<FileDiff> {
        let from_ref = self.get_merged_base_ref(project_path, base_branch, merge_commit_sha);
        self.get_file_diff_between_refs(file_path, project_path, &from_ref, merge_commit_sha)
    }

    fn get_file_content_at_ref(
        &self,
        project_path: &str,
        git_ref: &str,
        file_path: &str,
    ) -> Option<String> {
        let output = Command::new("git")
            .args(["show", &format!("{}:{}", git_ref, file_path)])
            .current_dir(project_path)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }
}

/// Get programming language from file path
fn get_language_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "ts" | "tsx" => "typescript".to_string(),
        "js" | "jsx" => "javascript".to_string(),
        "rs" => "rust".to_string(),
        "py" => "python".to_string(),
        "go" => "go".to_string(),
        "java" => "java".to_string(),
        "c" | "h" => "c".to_string(),
        "cpp" | "hpp" | "cc" => "cpp".to_string(),
        "rb" => "ruby".to_string(),
        "php" => "php".to_string(),
        "swift" => "swift".to_string(),
        "kt" => "kotlin".to_string(),
        "md" => "markdown".to_string(),
        "json" => "json".to_string(),
        "yaml" | "yml" => "yaml".to_string(),
        "toml" => "toml".to_string(),
        "html" => "html".to_string(),
        "css" => "css".to_string(),
        "scss" | "sass" => "scss".to_string(),
        "sql" => "sql".to_string(),
        "sh" | "bash" | "zsh" => "bash".to_string(),
        _ => "plaintext".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_from_path() {
        assert_eq!(get_language_from_path("src/app.ts"), "typescript");
        assert_eq!(get_language_from_path("src/app.tsx"), "typescript");
        assert_eq!(get_language_from_path("main.rs"), "rust");
        assert_eq!(get_language_from_path("app.py"), "python");
        assert_eq!(get_language_from_path("config.json"), "json");
        assert_eq!(get_language_from_path("README.md"), "markdown");
        assert_eq!(get_language_from_path("unknown"), "plaintext");
    }
}
