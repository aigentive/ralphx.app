use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::application::git_service::GitService;
use crate::domain::entities::{
    AgentConversationWorkspace, AgentConversationWorkspaceMode, ChatConversationId,
    IdeationAnalysisBaseRefKind, Project, Task,
};
use crate::domain::state_machine::transition_handler::run_pre_execution_setup;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct AgentConversationWorkspaceBaseSelection {
    pub kind: Option<IdeationAnalysisBaseRefKind>,
    pub base_ref: Option<String>,
    pub display_name: Option<String>,
}

pub async fn prepare_agent_conversation_workspace(
    project: &Project,
    conversation_id: &ChatConversationId,
    mode: AgentConversationWorkspaceMode,
    selection: AgentConversationWorkspaceBaseSelection,
) -> AppResult<AgentConversationWorkspace> {
    let repo_path = PathBuf::from(&project.working_directory);
    let current_branch = GitService::get_current_branch(&repo_path)
        .await
        .ok()
        .filter(|branch| branch != "HEAD");
    let project_default =
        GitService::resolve_project_default_branch(&repo_path, project.base_branch.as_deref())
            .await;
    let kind = selection.kind.unwrap_or_else(|| {
        if current_branch
            .as_deref()
            .is_some_and(|branch| branch != project_default)
        {
            IdeationAnalysisBaseRefKind::CurrentBranch
        } else {
            IdeationAnalysisBaseRefKind::ProjectDefault
        }
    });

    if kind == IdeationAnalysisBaseRefKind::PullRequest {
        return Err(AppError::Validation(
            "PR-backed agent conversation base refs require PR workspace provisioning and are not enabled in this slice"
                .to_string(),
        ));
    }

    let base_ref = match kind {
        IdeationAnalysisBaseRefKind::ProjectDefault => selection
            .base_ref
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(project_default),
        IdeationAnalysisBaseRefKind::CurrentBranch => selection
            .base_ref
            .filter(|value| !value.trim().is_empty())
            .or(current_branch)
            .ok_or_else(|| AppError::Validation("Unable to resolve current branch".to_string()))?,
        IdeationAnalysisBaseRefKind::LocalBranch => selection
            .base_ref
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation(
                    "Local branch agent conversation base requires base_ref".to_string(),
                )
            })?,
        IdeationAnalysisBaseRefKind::PullRequest => unreachable!("handled above"),
    };

    if !GitService::ref_exists(&repo_path, &base_ref).await? {
        return Err(AppError::Validation(format!(
            "Agent conversation base ref '{}' does not exist in the project repository",
            base_ref
        )));
    }

    let display_name = selection
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match kind {
            IdeationAnalysisBaseRefKind::ProjectDefault => format!("Project default ({base_ref})"),
            IdeationAnalysisBaseRefKind::CurrentBranch => format!("Current branch ({base_ref})"),
            IdeationAnalysisBaseRefKind::LocalBranch => base_ref.clone(),
            IdeationAnalysisBaseRefKind::PullRequest => base_ref.clone(),
        });
    let branch_name = agent_conversation_branch_name(project, conversation_id);
    let worktree_path = resolve_agent_conversation_workspace_path(project, conversation_id)?;

    ensure_agent_conversation_worktree(&repo_path, &worktree_path, &branch_name, &base_ref).await?;
    run_agent_conversation_workspace_setup(project, conversation_id, &worktree_path, &branch_name)
        .await;
    let base_commit = GitService::get_head_sha(&worktree_path).await?;

    Ok(AgentConversationWorkspace {
        conversation_id: conversation_id.clone(),
        project_id: project.id.clone(),
        mode,
        base_ref_kind: kind,
        base_ref,
        base_display_name: Some(display_name),
        base_commit: Some(base_commit),
        branch_name,
        worktree_path: worktree_path.to_string_lossy().to_string(),
        linked_ideation_session_id: None,
        linked_plan_branch_id: None,
        publication_pr_number: None,
        publication_pr_url: None,
        publication_pr_status: None,
        publication_push_status: None,
        status: crate::domain::entities::AgentConversationWorkspaceStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

async fn run_agent_conversation_workspace_setup(
    project: &Project,
    conversation_id: &ChatConversationId,
    worktree_path: &Path,
    branch_name: &str,
) {
    let conversation_id_str = conversation_id.as_str();
    let mut setup_task = Task::new(
        project.id.clone(),
        format!("Agent conversation {conversation_id_str}"),
    );
    setup_task.task_branch = Some(branch_name.to_string());
    setup_task.worktree_path = Some(worktree_path.to_string_lossy().to_string());

    let Some(result) = run_pre_execution_setup(
        project,
        &setup_task,
        worktree_path,
        &conversation_id_str,
        None,
        "agent_conversation_setup",
        &tokio_util::sync::CancellationToken::new(),
    )
    .await
    else {
        return;
    };

    if result.success {
        tracing::info!(
            conversation_id = %conversation_id,
            worktree_path = %worktree_path.display(),
            command_count = result.log.len(),
            "Agent conversation worktree setup completed"
        );
    } else {
        tracing::warn!(
            conversation_id = %conversation_id,
            worktree_path = %worktree_path.display(),
            command_count = result.log.len(),
            "Agent conversation worktree setup had failures; continuing with workspace launch"
        );
    }
}

pub fn agent_conversation_branch_name(
    project: &Project,
    conversation_id: &ChatConversationId,
) -> String {
    let project_slug = slug_branch_component(&project.name);
    let short_id = conversation_id
        .as_str()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>();
    let short_id = if short_id.is_empty() {
        "conversation".to_string()
    } else {
        short_id
    };
    format!("ralphx/{project_slug}/agent-{short_id}")
}

async fn ensure_agent_conversation_worktree(
    repo_path: &Path,
    workspace_path: &Path,
    branch_name: &str,
    base_ref: &str,
) -> AppResult<()> {
    if workspace_path.exists() {
        if !workspace_path.is_dir() {
            return Err(AppError::Validation(format!(
                "Agent conversation workspace path exists but is not a directory: {}",
                workspace_path.display()
            )));
        }
        let checked_out = GitService::get_current_branch(workspace_path).await?;
        if checked_out != branch_name {
            return Err(AppError::Validation(format!(
                "Existing agent conversation workspace {} is checked out at '{}' instead of '{}'",
                workspace_path.display(),
                checked_out,
                branch_name
            )));
        }
        return Ok(());
    }

    if GitService::branch_exists(repo_path, branch_name).await? {
        GitService::checkout_existing_branch_worktree(repo_path, workspace_path, branch_name).await
    } else {
        GitService::create_worktree(repo_path, workspace_path, branch_name, base_ref).await
    }
}

pub fn resolve_agent_conversation_workspace_path(
    project: &Project,
    conversation_id: &ChatConversationId,
) -> AppResult<PathBuf> {
    let parent = expand_worktree_parent(project.worktree_parent_or_default())?;
    Ok(parent
        .join(hashed_path_component("project", project.id.as_str()))
        .join(hashed_path_component(
            "agent-conversation",
            &conversation_id.as_str(),
        )))
}

pub async fn resolve_valid_agent_conversation_workspace_path(
    project: &Project,
    workspace: &AgentConversationWorkspace,
) -> AppResult<PathBuf> {
    if workspace.project_id != project.id {
        return Err(AppError::Validation(format!(
            "Agent conversation workspace {} belongs to project {} instead of {}",
            workspace.conversation_id, workspace.project_id, project.id
        )));
    }

    let expected_path =
        resolve_agent_conversation_workspace_path(project, &workspace.conversation_id)?;
    let stored_path = PathBuf::from(&workspace.worktree_path);
    if stored_path != expected_path {
        return Err(AppError::Validation(format!(
            "Agent conversation workspace path mismatch for conversation {}",
            workspace.conversation_id
        )));
    }

    let project_root = PathBuf::from(&project.working_directory);
    if expected_path == project_root {
        return Err(AppError::Validation(format!(
            "Agent conversation workspace {} points to the project root",
            workspace.conversation_id
        )));
    }

    if !expected_path.is_dir() {
        return Err(AppError::Validation(format!(
            "Agent conversation workspace is missing: {}",
            expected_path.display()
        )));
    }

    let checked_out = GitService::get_current_branch(&expected_path).await?;
    if checked_out != workspace.branch_name {
        return Err(AppError::Validation(format!(
            "Agent conversation workspace {} is checked out at '{}' instead of '{}'",
            workspace.conversation_id, checked_out, workspace.branch_name
        )));
    }

    Ok(expected_path)
}

fn expand_worktree_parent(parent: &str) -> AppResult<PathBuf> {
    let expanded = if let Some(rest) = parent.strip_prefix("~/") {
        let home = dirs::home_dir().ok_or_else(|| {
            AppError::Validation(
                "Cannot expand worktree parent because home directory is unavailable".to_string(),
            )
        })?;
        home.join(rest)
    } else {
        PathBuf::from(parent)
    };

    if !expanded.is_absolute()
        || expanded
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::Validation(format!(
            "Invalid agent conversation worktree parent path: {}",
            expanded.display()
        )));
    }

    Ok(expanded)
}

fn hashed_path_component(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hash = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}-{hash}")
}

fn slug_branch_component(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        AgentConversationWorkspaceMode, ChatConversationId, IdeationAnalysisBaseRefKind, Project,
    };
    use std::process::Command;

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git command should spawn");
        assert!(
            output.status.success(),
            "git {:?} failed: {}{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn setup_repo(root: &Path) {
        std::fs::create_dir_all(root).expect("repo root should be created");
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.email", "test@example.com"]);
        git(root, &["config", "user.name", "Test User"]);
        std::fs::write(root.join("README.md"), "hello\n").expect("fixture file should be written");
        git(root, &["add", "README.md"]);
        git(root, &["commit", "-m", "initial"]);
    }

    #[tokio::test]
    async fn prepare_agent_conversation_workspace_runs_project_worktree_setup() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let repo_path = temp.path().join("repo");
        let worktree_parent = temp.path().join("worktrees");
        setup_repo(&repo_path);

        let mut project = Project::new(
            "Agent Setup".to_string(),
            repo_path.to_string_lossy().to_string(),
        );
        project.worktree_parent_directory = Some(worktree_parent.to_string_lossy().to_string());
        project.custom_analysis = Some(
            r#"[{"path": ".", "label": "Agent setup", "worktree_setup": ["touch .agent_setup_marker"]}]"#
                .to_string(),
        );

        let conversation_id =
            ChatConversationId::from_string("conversation-setup-test".to_string());
        let workspace = prepare_agent_conversation_workspace(
            &project,
            &conversation_id,
            AgentConversationWorkspaceMode::Edit,
            AgentConversationWorkspaceBaseSelection {
                kind: Some(IdeationAnalysisBaseRefKind::ProjectDefault),
                base_ref: Some("main".to_string()),
                display_name: None,
            },
        )
        .await
        .expect("workspace should be prepared");

        assert!(
            Path::new(&workspace.worktree_path)
                .join(".agent_setup_marker")
                .exists(),
            "agent conversation worktree should run project worktree_setup commands"
        );
        let captured_head =
            GitService::get_head_sha(Path::new(&workspace.worktree_path))
                .await
                .expect("workspace HEAD should resolve");
        assert_eq!(
            workspace.base_commit.as_deref(),
            Some(captured_head.as_str()),
            "agent conversation workspace should always capture the immutable base commit"
        );
    }
}
