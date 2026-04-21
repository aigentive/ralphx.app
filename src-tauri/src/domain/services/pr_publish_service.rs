use std::path::Path;
use std::sync::Arc;

use tempfile::NamedTempFile;

use crate::domain::entities::{ArtifactContent, PlanBranch, Project, Task, TaskCategory};
use crate::domain::repositories::{ArtifactRepository, IdeationSessionRepository};
use crate::domain::services::GithubServiceTrait;
use crate::error::{AppError, AppResult};

const GITHUB_PR_BODY_SOFT_LIMIT_CHARS: usize = 60_000;
const PR_BODY_TRUNCATION_NOTICE: &str =
    "\n\n_Excerpt truncated by RalphX because GitHub PR descriptions have a body size limit._";
const RALPHX_REPOSITORY_URL: &str = "https://github.com/aigentive/ralphx";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrReviewState {
    Draft,
    Ready,
}

pub struct PlanPrPublisher<'a> {
    github: &'a Arc<dyn GithubServiceTrait>,
    ideation_session_repo: Option<&'a Arc<dyn IdeationSessionRepository>>,
    artifact_repo: Option<&'a Arc<dyn ArtifactRepository>>,
}

impl<'a> PlanPrPublisher<'a> {
    pub fn new(
        github: &'a Arc<dyn GithubServiceTrait>,
        ideation_session_repo: Option<&'a Arc<dyn IdeationSessionRepository>>,
        artifact_repo: Option<&'a Arc<dyn ArtifactRepository>>,
    ) -> Self {
        Self {
            github,
            ideation_session_repo,
            artifact_repo,
        }
    }

    pub async fn create_draft_pr(
        &self,
        task: &Task,
        project: &Project,
        plan_branch: &PlanBranch,
    ) -> AppResult<(i64, String)> {
        let repo_path = Path::new(&project.working_directory);
        let title = self
            .build_title(task, plan_branch, PrReviewState::Draft)
            .await;
        let body_file = self
            .write_body_file(task, project, plan_branch, PrReviewState::Draft)
            .await?;
        let base = resolve_plan_branch_pr_base(project, plan_branch);

        self.github
            .create_draft_pr(
                repo_path,
                &base,
                &plan_branch.branch_name,
                &title,
                body_file.path(),
            )
            .await
    }

    pub async fn sync_existing_pr(
        &self,
        task: &Task,
        project: &Project,
        plan_branch: &PlanBranch,
        review_state: PrReviewState,
    ) -> AppResult<()> {
        let Some(pr_number) = plan_branch.pr_number else {
            return Ok(());
        };

        let repo_path = Path::new(&project.working_directory);
        let title = self.build_title(task, plan_branch, review_state).await;
        let body_file = self
            .write_body_file(task, project, plan_branch, review_state)
            .await?;

        self.github
            .update_pr_details(repo_path, pr_number, &title, body_file.path())
            .await
    }

    async fn write_body_file(
        &self,
        task: &Task,
        project: &Project,
        plan_branch: &PlanBranch,
        review_state: PrReviewState,
    ) -> AppResult<NamedTempFile> {
        let body = self
            .build_body(task, project, plan_branch, review_state)
            .await;
        let body_file = NamedTempFile::new().map_err(|e| {
            AppError::Infrastructure(format!("failed to create PR body temp file: {e}"))
        })?;
        use std::io::Write as _;
        (&body_file).write_all(body.as_bytes()).map_err(|e| {
            AppError::Infrastructure(format!("failed to write PR body temp file: {e}"))
        })?;
        Ok(body_file)
    }

    async fn build_title(
        &self,
        task: &Task,
        plan_branch: &PlanBranch,
        review_state: PrReviewState,
    ) -> String {
        let display_title = self.resolve_display_title(task, plan_branch).await;
        match review_state {
            PrReviewState::Draft => format!("Plan: {}", display_title.trim()),
            PrReviewState::Ready => display_title.trim().to_string(),
        }
    }

    async fn build_body(
        &self,
        task: &Task,
        project: &Project,
        plan_branch: &PlanBranch,
        review_state: PrReviewState,
    ) -> String {
        let repo_path = Path::new(&project.working_directory);
        let pr_base = resolve_plan_branch_pr_base(project, plan_branch);
        let template = read_pull_request_template(repo_path).await;
        let plan_markdown = self
            .read_plan_artifact_markdown(plan_branch)
            .await
            .unwrap_or_else(|| {
                "_No plan artifact was available when RalphX synced this PR._".to_string()
            });

        let mut sections = Vec::new();
        if let Some(template) = template {
            sections.push(template);
        }

        let state_line = match review_state {
            PrReviewState::Draft => {
                "Draft while RalphX is still merging plan tasks into the plan branch."
            }
            PrReviewState::Ready => {
                "Ready for GitHub review. RalphX has finished merging plan tasks into the plan branch."
            }
        };
        let workflow_task_note = if task.category == TaskCategory::PlanMerge {
            format!(
                "{}. This is the final workflow task that hands the completed plan branch to GitHub review.",
                task.title.trim()
            )
        } else {
            task.title.trim().to_string()
        };

        sections.push(format!(
            "## RalphX Status\n\n- State: {}\n- Current RalphX task: **{}**\n- Base branch: `{}`\n- Plan branch: `{}`",
            state_line,
            workflow_task_note,
            pr_base,
            plan_branch.branch_name
        ));

        sections.push(
            "## How To Review\n\n\
             - Review the plan below against the delivered diff and repository checks.\n\
             - Leave GitHub comments or requested changes if the implementation does not satisfy the plan.\n\
             - Merge this PR in GitHub when it is ready; RalphX will detect the merge and finish the plan."
                .to_string(),
        );

        let footer = format!("---\n\n_Generated by [RalphX]({})_", RALPHX_REPOSITORY_URL);
        let prefix = format!("{}\n\n## Plan\n\n", sections.join("\n\n"));
        let suffix = format!("\n\n{footer}");

        fit_plan_markdown_to_pr_body(&prefix, &plan_markdown, &suffix)
    }

    async fn resolve_display_title(&self, task: &Task, plan_branch: &PlanBranch) -> String {
        if let Some(repo) = self.ideation_session_repo {
            if let Ok(Some(session)) = repo.get_by_id(&plan_branch.session_id).await {
                if let Some(title) = session.title.filter(|title| !title.trim().is_empty()) {
                    return title.trim().to_string();
                }
            }
        }

        if let Some(repo) = self.artifact_repo {
            if let Ok(Some(artifact)) = repo.get_by_id(&plan_branch.plan_artifact_id).await {
                if !artifact.name.trim().is_empty() {
                    return artifact.name.trim().to_string();
                }
            }
        }

        if !task.title.trim().is_empty() {
            return task.title.trim().to_string();
        }

        plan_branch.branch_name.clone()
    }

    async fn read_plan_artifact_markdown(&self, plan_branch: &PlanBranch) -> Option<String> {
        let repo = self.artifact_repo?;
        let artifact = repo
            .get_by_id(&plan_branch.plan_artifact_id)
            .await
            .ok()
            .flatten()?;
        let raw = match artifact.content {
            ArtifactContent::Inline { text } => text,
            ArtifactContent::File { path } => tokio::fs::read_to_string(path).await.ok()?,
        };

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        Some(trimmed.to_string())
    }
}

async fn read_pull_request_template(repo_path: &Path) -> Option<String> {
    let template_path = repo_path.join(".github").join("PULL_REQUEST_TEMPLATE.md");
    if !template_path.exists() {
        return None;
    }

    match tokio::fs::read_to_string(&template_path).await {
        Ok(content) if !content.trim().is_empty() => Some(content.trim().to_string()),
        _ => None,
    }
}

fn resolve_plan_branch_pr_base(project: &Project, plan_branch: &PlanBranch) -> String {
    plan_branch
        .base_branch_override
        .clone()
        .or_else(|| project.base_branch.clone())
        .unwrap_or_else(|| plan_branch.source_branch.clone())
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn fit_plan_markdown_to_pr_body(prefix: &str, plan_markdown: &str, suffix: &str) -> String {
    let full_body = format!("{prefix}{plan_markdown}{suffix}");
    if char_count(&full_body) <= GITHUB_PR_BODY_SOFT_LIMIT_CHARS {
        return full_body;
    }

    let fixed_chars =
        char_count(prefix) + char_count(suffix) + char_count(PR_BODY_TRUNCATION_NOTICE);
    if fixed_chars >= GITHUB_PR_BODY_SOFT_LIMIT_CHARS {
        return truncate_chars(&full_body, GITHUB_PR_BODY_SOFT_LIMIT_CHARS);
    }

    let available_plan_chars = GITHUB_PR_BODY_SOFT_LIMIT_CHARS - fixed_chars;
    let truncated_plan = truncate_chars(plan_markdown, available_plan_chars);
    format!(
        "{prefix}{}{}{suffix}",
        truncated_plan.trim_end(),
        PR_BODY_TRUNCATION_NOTICE
    )
}
