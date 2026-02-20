// Memory-based ReviewIssueRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage without a real database

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{
    IssueProgressSummary, IssueStatus, ReviewIssueEntity as ReviewIssue, ReviewIssueId,
    SeverityBreakdown, SeverityCount, TaskId,
};
use crate::error::AppResult;
use crate::infrastructure::sqlite::ReviewIssueRepository;

/// In-memory implementation of ReviewIssueRepository for testing
pub struct MemoryReviewIssueRepository {
    issues: Arc<RwLock<HashMap<ReviewIssueId, ReviewIssue>>>,
}

impl Default for MemoryReviewIssueRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryReviewIssueRepository {
    /// Create a new empty in-memory review issue repository
    pub fn new() -> Self {
        Self {
            issues: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ReviewIssueRepository for MemoryReviewIssueRepository {
    async fn create(&self, issue: ReviewIssue) -> AppResult<ReviewIssue> {
        let mut issues = self.issues.write().await;
        issues.insert(issue.id.clone(), issue.clone());
        Ok(issue)
    }

    async fn bulk_create(&self, issues_to_create: Vec<ReviewIssue>) -> AppResult<Vec<ReviewIssue>> {
        let mut issues = self.issues.write().await;
        let mut created = Vec::new();

        for issue in issues_to_create {
            issues.insert(issue.id.clone(), issue.clone());
            created.push(issue);
        }

        Ok(created)
    }

    async fn get_by_id(&self, id: &ReviewIssueId) -> AppResult<Option<ReviewIssue>> {
        let issues = self.issues.read().await;
        Ok(issues.get(id).cloned())
    }

    async fn get_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let issues = self.issues.read().await;
        let result: Vec<ReviewIssue> = issues
            .values()
            .filter(|i| i.task_id == *task_id)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn get_open_by_task_id(&self, task_id: &TaskId) -> AppResult<Vec<ReviewIssue>> {
        let issues = self.issues.read().await;
        let result: Vec<ReviewIssue> = issues
            .values()
            .filter(|i| i.task_id == *task_id && i.status == IssueStatus::Open)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn update_status(
        &self,
        id: &ReviewIssueId,
        status: IssueStatus,
        resolution_notes: Option<String>,
    ) -> AppResult<ReviewIssue> {
        let mut issues = self.issues.write().await;
        if let Some(issue) = issues.get_mut(id) {
            issue.status = status;
            if let Some(notes) = resolution_notes {
                issue.resolution_notes = Some(notes);
            }
            issue.touch();
            Ok(issue.clone())
        } else {
            Err(crate::error::AppError::NotFound(format!(
                "Issue {} not found",
                id.as_str()
            )))
        }
    }

    async fn update(&self, issue: &ReviewIssue) -> AppResult<()> {
        let mut issues = self.issues.write().await;
        issues.insert(issue.id.clone(), issue.clone());
        Ok(())
    }

    async fn get_summary(&self, task_id: &TaskId) -> AppResult<IssueProgressSummary> {
        let issues = self.issues.read().await;
        let task_issues: Vec<&ReviewIssue> =
            issues.values().filter(|i| i.task_id == *task_id).collect();

        let total = task_issues.len() as u32;
        let mut open = 0u32;
        let mut in_progress = 0u32;
        let mut addressed = 0u32;
        let mut verified = 0u32;
        let mut wontfix = 0u32;

        let mut critical = SeverityCount::default();
        let mut major = SeverityCount::default();
        let mut minor = SeverityCount::default();
        let mut suggestion = SeverityCount::default();

        for issue in &task_issues {
            match issue.status {
                IssueStatus::Open => open += 1,
                IssueStatus::InProgress => in_progress += 1,
                IssueStatus::Addressed => addressed += 1,
                IssueStatus::Verified => verified += 1,
                IssueStatus::WontFix => wontfix += 1,
            }

            let severity_count = match issue.severity {
                crate::domain::entities::IssueSeverity::Critical => &mut critical,
                crate::domain::entities::IssueSeverity::Major => &mut major,
                crate::domain::entities::IssueSeverity::Minor => &mut minor,
                crate::domain::entities::IssueSeverity::Suggestion => &mut suggestion,
            };

            severity_count.total += 1;
            if issue.status == IssueStatus::Open {
                severity_count.open += 1;
            } else if issue.status.is_resolved() {
                severity_count.resolved += 1;
            }
        }

        let resolved = addressed + verified + wontfix;
        let percent_resolved = if total > 0 {
            (resolved as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Ok(IssueProgressSummary {
            task_id: task_id.as_str().to_string(),
            total,
            open,
            in_progress,
            addressed,
            verified,
            wontfix,
            percent_resolved,
            by_severity: SeverityBreakdown {
                critical,
                major,
                minor,
                suggestion,
            },
        })
    }
}

#[cfg(test)]
#[path = "memory_review_issue_repo_tests.rs"]
mod tests;
