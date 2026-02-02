//! Diff Service - Extracts file changes from agent activity and git
//!
//! Provides file change information for the DiffViewer by:
//! 1. Querying activity events to find Write/Edit tool calls
//! 2. Using git to get actual diff content

use crate::domain::entities::{ActivityEventType, TaskId};
use crate::domain::repositories::{ActivityEventFilter, ActivityEventRepository};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;

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
pub struct DiffService {
    activity_repo: Arc<dyn ActivityEventRepository>,
}

impl DiffService {
    pub fn new(activity_repo: Arc<dyn ActivityEventRepository>) -> Self {
        Self { activity_repo }
    }

    /// Get all files changed by the agent for a task
    /// Compares against base_branch to show all changes since branching
    pub async fn get_task_file_changes(
        &self,
        task_id: &TaskId,
        project_path: &str,
        base_branch: &str,
    ) -> AppResult<Vec<FileChange>> {
        // Get all tool_call events for this task
        let filter = ActivityEventFilter::new()
            .with_event_types(vec![ActivityEventType::ToolCall]);

        // Fetch all tool call events (paginated, but we'll get all pages)
        let mut all_events = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let page = self
                .activity_repo
                .list_by_task_id(task_id, cursor.as_deref(), 100, Some(&filter))
                .await?;

            all_events.extend(page.events);

            if !page.has_more {
                break;
            }
            cursor = page.cursor;
        }

        // Extract file paths from Write/Edit tool calls
        let mut modified_files = HashSet::new();

        for event in &all_events {
            // Parse the tool call metadata (not content - content is human-readable string)
            // Metadata format: { "tool_name": "Write", "arguments": {...} }
            if let Some(ref metadata) = event.metadata {
                if let Ok(tool_meta) = serde_json::from_str::<ToolCallMetadata>(metadata) {
                    let tool_name = tool_meta.tool_name.to_lowercase();

                    if tool_name == "write" || tool_name == "edit" {
                        if let Some(file_path) = tool_meta.arguments.get("file_path") {
                            if let Some(path_str) = file_path.as_str() {
                                // Convert absolute path to relative if needed
                                let relative_path = if path_str.starts_with(project_path) {
                                    path_str
                                        .strip_prefix(project_path)
                                        .unwrap_or(path_str)
                                        .trim_start_matches('/')
                                        .to_string()
                                } else {
                                    path_str.to_string()
                                };
                                modified_files.insert(relative_path);
                            }
                        }
                    }
                }
            }
        }

        // For each file, determine its status using git
        let mut changes = Vec::new();
        for file_path in modified_files {
            if let Some(change) = self.get_file_change_status(&file_path, project_path, base_branch)
            {
                changes.push(change);
            }
        }

        // Sort by path for consistent ordering
        changes.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(changes)
    }

    /// Get the change status for a single file using git
    /// Compares against base_branch to show all changes since branching
    fn get_file_change_status(
        &self,
        file_path: &str,
        project_path: &str,
        base_branch: &str,
    ) -> Option<FileChange> {
        // First check for tracked changes using git diff against base branch
        let output = Command::new("git")
            .args(["diff", "--numstat", base_branch, "--", file_path])
            .current_dir(project_path)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let additions: u32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
                let deletions: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);

                // Determine status
                let status = if additions > 0 && deletions == 0 {
                    // Check if file existed in base branch using ls-tree
                    let existed = Command::new("git")
                        .args(["ls-tree", "-r", "--name-only", base_branch, "--", file_path])
                        .current_dir(project_path)
                        .output()
                        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
                        .unwrap_or(false);

                    if existed {
                        FileChangeStatus::Modified
                    } else {
                        FileChangeStatus::Added
                    }
                } else if additions == 0 && deletions > 0 {
                    FileChangeStatus::Deleted
                } else {
                    FileChangeStatus::Modified
                };

                return Some(FileChange {
                    path: file_path.to_string(),
                    status,
                    additions,
                    deletions,
                });
            }
        }

        // File might be untracked (new file not yet added to git)
        let untracked = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard", file_path])
            .current_dir(project_path)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&untracked.stdout);
        if !stdout.trim().is_empty() {
            // Count lines in the new file
            let full_path = std::path::Path::new(project_path).join(file_path);
            let content = std::fs::read_to_string(&full_path).unwrap_or_default();
            let additions = content.lines().count() as u32;

            return Some(FileChange {
                path: file_path.to_string(),
                status: FileChangeStatus::Added,
                additions,
                deletions: 0,
            });
        }

        None
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

        // Get current content
        let new_content = std::fs::read_to_string(&full_path).unwrap_or_default();

        // Get old content from base branch
        let old_output = Command::new("git")
            .args(["show", &format!("{}:{}", base_branch, file_path)])
            .current_dir(project_path)
            .output();

        let old_content = old_output
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Determine language from file extension
        let language = get_language_from_path(file_path);

        Ok(FileDiff {
            file_path: file_path.to_string(),
            old_content,
            new_content,
            language,
        })
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
        // Get old content from parent commit (commit^)
        let old_output = Command::new("git")
            .args(["show", &format!("{}^:{}", commit_sha, file_path)])
            .current_dir(project_path)
            .output();

        let old_content = old_output
            .map(|o| {
                if o.status.success() {
                    String::from_utf8_lossy(&o.stdout).to_string()
                } else {
                    // File didn't exist in parent (new file)
                    String::new()
                }
            })
            .unwrap_or_default();

        // Get new content from this commit
        let new_output = Command::new("git")
            .args(["show", &format!("{}:{}", commit_sha, file_path)])
            .current_dir(project_path)
            .output();

        let new_content = new_output
            .map(|o| {
                if o.status.success() {
                    String::from_utf8_lossy(&o.stdout).to_string()
                } else {
                    // File was deleted in this commit
                    String::new()
                }
            })
            .unwrap_or_default();

        // Determine language from file extension
        let language = get_language_from_path(file_path);

        Ok(FileDiff {
            file_path: file_path.to_string(),
            old_content,
            new_content,
            language,
        })
    }
}

/// Parsed tool call from activity event metadata
/// Format: { "tool_name": "Write", "arguments": {...} }
#[derive(Debug, Deserialize)]
struct ToolCallMetadata {
    tool_name: String,
    arguments: serde_json::Value,
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
