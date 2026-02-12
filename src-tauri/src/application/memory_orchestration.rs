// Memory pipeline orchestration
//
// Triggers background memory-maintainer and memory-capture agents
// after agent run completion based on context type and project settings.

use crate::domain::entities::{ChatContextType, ChatConversationId, ProjectId};
use crate::infrastructure::agents::claude::build_spawnable_command;
use std::path::Path;

/// Memory category derived from chat context type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryCategory {
    Planning,
    Execution,
    Review,
    Merge,
    ProjectChat,
}

impl MemoryCategory {
    /// Map ChatContextType to MemoryCategory
    pub fn from_context_type(context_type: ChatContextType) -> Self {
        match context_type {
            ChatContextType::Ideation => MemoryCategory::Planning,
            ChatContextType::Task | ChatContextType::TaskExecution => MemoryCategory::Execution,
            ChatContextType::Review => MemoryCategory::Review,
            ChatContextType::Merge => MemoryCategory::Merge,
            ChatContextType::Project => MemoryCategory::ProjectChat,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryCategory::Planning => "planning",
            MemoryCategory::Execution => "execution",
            MemoryCategory::Review => "review",
            MemoryCategory::Merge => "merge",
            MemoryCategory::ProjectChat => "project_chat",
        }
    }
}

/// Project memory settings (stub - will be replaced with repository-backed implementation)
#[derive(Debug, Clone)]
pub struct ProjectMemorySettings {
    pub enabled: bool,
    pub maintenance_categories: Vec<String>,
    pub capture_categories: Vec<String>,
}

impl Default for ProjectMemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            maintenance_categories: vec![
                "execution".to_string(),
                "review".to_string(),
                "merge".to_string(),
            ],
            capture_categories: vec![
                "planning".to_string(),
                "execution".to_string(),
                "review".to_string(),
            ],
        }
    }
}

/// Determine which memory pipelines should be triggered for a given context.
///
/// Returns (should_maintain, should_capture) based on settings and context.
/// This is the core logic extracted for testability.
pub fn resolve_pipelines(
    context_type: ChatContextType,
    project_id: Option<&ProjectId>,
    agent_name: Option<&str>,
    settings: &ProjectMemorySettings,
) -> Option<(bool, bool)> {
    // Guard: If no project ID, skip (memory is project-scoped)
    if project_id.is_none() {
        tracing::debug!("resolve_pipelines: no project_id, skipping");
        return None;
    }

    // Recursion guard: Skip if current agent is a memory agent
    if let Some(name) = agent_name {
        if name == "memory-maintainer" || name == "memory-capture" {
            tracing::debug!(
                agent_name = name,
                "resolve_pipelines: recursion guard triggered, skipping"
            );
            return None;
        }
    }

    // Early exit if memory disabled
    if !settings.enabled {
        tracing::debug!("resolve_pipelines: memory disabled for project, skipping");
        return None;
    }

    // Map context to category
    let category = MemoryCategory::from_context_type(context_type);
    let category_str = category.as_str();

    let should_maintain = settings
        .maintenance_categories
        .contains(&category_str.to_string());
    let should_capture = settings
        .capture_categories
        .contains(&category_str.to_string());

    if !should_maintain && !should_capture {
        tracing::debug!(
            category = category_str,
            "resolve_pipelines: category not in any enabled categories, skipping"
        );
        return None;
    }

    Some((should_maintain, should_capture))
}

/// Trigger memory pipelines after agent run completion
///
/// This function orchestrates background memory agents based on:
/// - Project memory settings (enabled/disabled, category filters)
/// - Context type (mapped to memory category)
/// - Recursion guard (skip if current agent is a memory agent)
///
/// Failures are logged but do not block the primary user workflow.
#[allow(clippy::too_many_arguments)]
pub async fn trigger_memory_pipelines(
    context_type: ChatContextType,
    context_id: &str,
    conversation_id: &ChatConversationId,
    project_id: Option<&ProjectId>,
    agent_name: Option<&str>,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
    settings: Option<ProjectMemorySettings>,
) {
    tracing::debug!(
        %context_type,
        context_id = %context_id,
        conversation_id = conversation_id.as_str(),
        "trigger_memory_pipelines: entry"
    );

    let proj_id = match project_id {
        Some(id) => id,
        None => {
            tracing::debug!("trigger_memory_pipelines: no project_id, skipping");
            return;
        }
    };

    // Use provided settings or defaults
    let settings = settings.unwrap_or_default();

    let Some((should_maintain, should_capture)) =
        resolve_pipelines(context_type, project_id, agent_name, &settings)
    else {
        return;
    };

    let category = MemoryCategory::from_context_type(context_type);

    tracing::info!(
        %context_type,
        category = category.as_str(),
        project_id = proj_id.as_str(),
        "trigger_memory_pipelines: mapped context to category"
    );

    // Spawn memory agents in parallel (fire-and-forget)
    let mut spawn_tasks = vec![];

    if should_maintain {
        let conv_id = conversation_id.clone();
        let ctx = context_type;
        let ctx_id = context_id.to_string();
        let proj = proj_id.clone();
        let cli = cli_path.to_path_buf();
        let plugin = plugin_dir.to_path_buf();
        let wd = working_directory.to_path_buf();

        spawn_tasks.push(tokio::spawn(async move {
            if let Err(e) = spawn_memory_maintainer(
                &conv_id,
                ctx,
                &ctx_id,
                &proj,
                &cli,
                &plugin,
                &wd,
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    conversation_id = conv_id.as_str(),
                    "trigger_memory_pipelines: failed to spawn memory-maintainer"
                );
                // TODO: Log to memory_events table when available
            }
        }));
    }

    if should_capture {
        let conv_id = conversation_id.clone();
        let ctx = context_type;
        let ctx_id = context_id.to_string();
        let proj = proj_id.clone();
        let cli = cli_path.to_path_buf();
        let plugin = plugin_dir.to_path_buf();
        let wd = working_directory.to_path_buf();

        spawn_tasks.push(tokio::spawn(async move {
            if let Err(e) = spawn_memory_capture(
                &conv_id,
                ctx,
                &ctx_id,
                &proj,
                &cli,
                &plugin,
                &wd,
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    conversation_id = conv_id.as_str(),
                    "trigger_memory_pipelines: failed to spawn memory-capture"
                );
                // TODO: Log to memory_events table when available
            }
        }));
    }

    tracing::info!(
        spawning_count = spawn_tasks.len(),
        maintenance = should_maintain,
        capture = should_capture,
        "trigger_memory_pipelines: spawning memory agents"
    );

    // Don't await - fire and forget
    // Tasks will log their own errors
}

/// Spawn memory-maintainer agent
///
/// Spawns the memory-maintainer agent with appropriate context and environment variables.
async fn spawn_memory_maintainer(
    conversation_id: &ChatConversationId,
    context_type: ChatContextType,
    context_id: &str,
    project_id: &ProjectId,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
) -> Result<(), String> {
    tracing::info!(
        conversation_id = conversation_id.as_str(),
        %context_type,
        context_id = %context_id,
        project_id = project_id.as_str(),
        "spawn_memory_maintainer: spawning agent"
    );

    let conv_id_str = conversation_id.as_str();
    let proj_id_str = project_id.as_str();
    let context_type_str = format!("{}", context_type);

    let prompt = format!(
        "Analyze and maintain memory rules for conversation_id='{}' in project_id='{}' (context: {}, {})",
        conv_id_str,
        proj_id_str,
        context_type,
        context_id
    );

    let mut cmd = build_spawnable_command(
        cli_path,
        plugin_dir,
        &prompt,
        Some("ralphx:memory-maintainer"),
        None,
        working_directory,
    )?;

    cmd.env("RALPHX_CONVERSATION_ID", &conv_id_str);
    cmd.env("RALPHX_CONTEXT_TYPE", &context_type_str);
    cmd.env("RALPHX_CONTEXT_ID", context_id);
    cmd.env("RALPHX_PROJECT_ID", proj_id_str);

    // Spawn and ignore the child process (fire-and-forget)
    let _child = cmd.spawn().await.map_err(|e| {
        format!("Failed to spawn memory-maintainer: {}", e)
    })?;

    Ok(())
}

/// Spawn memory-capture agent
///
/// Spawns the memory-capture agent with appropriate context and environment variables.
async fn spawn_memory_capture(
    conversation_id: &ChatConversationId,
    context_type: ChatContextType,
    context_id: &str,
    project_id: &ProjectId,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
) -> Result<(), String> {
    tracing::info!(
        conversation_id = conversation_id.as_str(),
        %context_type,
        context_id = %context_id,
        project_id = project_id.as_str(),
        "spawn_memory_capture: spawning agent"
    );

    let conv_id_str = conversation_id.as_str();
    let proj_id_str = project_id.as_str();
    let context_type_str = format!("{}", context_type);

    let prompt = format!(
        "Capture learning from conversation_id='{}' in project_id='{}' (context: {}, {})",
        conv_id_str,
        proj_id_str,
        context_type,
        context_id
    );

    let mut cmd = build_spawnable_command(
        cli_path,
        plugin_dir,
        &prompt,
        Some("ralphx:memory-capture"),
        None,
        working_directory,
    )?;

    cmd.env("RALPHX_CONVERSATION_ID", &conv_id_str);
    cmd.env("RALPHX_CONTEXT_TYPE", &context_type_str);
    cmd.env("RALPHX_CONTEXT_ID", context_id);
    cmd.env("RALPHX_PROJECT_ID", proj_id_str);

    // Spawn and ignore the child process (fire-and-forget)
    let _child = cmd.spawn().await.map_err(|e| {
        format!("Failed to spawn memory-capture: {}", e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_context_to_category_mapping() {
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::Ideation),
            MemoryCategory::Planning
        );
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::Task),
            MemoryCategory::Execution
        );
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::TaskExecution),
            MemoryCategory::Execution
        );
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::Review),
            MemoryCategory::Review
        );
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::Merge),
            MemoryCategory::Merge
        );
        assert_eq!(
            MemoryCategory::from_context_type(ChatContextType::Project),
            MemoryCategory::ProjectChat
        );
    }

    #[test]
    fn test_category_as_str() {
        assert_eq!(MemoryCategory::Planning.as_str(), "planning");
        assert_eq!(MemoryCategory::Execution.as_str(), "execution");
        assert_eq!(MemoryCategory::Review.as_str(), "review");
        assert_eq!(MemoryCategory::Merge.as_str(), "merge");
        assert_eq!(MemoryCategory::ProjectChat.as_str(), "project_chat");
    }

    #[test]
    fn test_default_settings() {
        let settings = ProjectMemorySettings::default();
        assert!(settings.enabled);
        assert!(settings.maintenance_categories.contains(&"execution".to_string()));
        assert!(settings.maintenance_categories.contains(&"review".to_string()));
        assert!(settings.maintenance_categories.contains(&"merge".to_string()));
        assert!(settings.capture_categories.contains(&"planning".to_string()));
        assert!(settings.capture_categories.contains(&"execution".to_string()));
        assert!(settings.capture_categories.contains(&"review".to_string()));
    }

    #[test]
    fn test_default_settings_maintenance_categories_count() {
        let settings = ProjectMemorySettings::default();
        assert_eq!(settings.maintenance_categories.len(), 3);
    }

    #[test]
    fn test_default_settings_capture_categories_count() {
        let settings = ProjectMemorySettings::default();
        assert_eq!(settings.capture_categories.len(), 3);
    }

    #[tokio::test]
    async fn test_trigger_memory_pipelines_no_project_id() {
        // Should return early without panicking
        let conv_id = ChatConversationId::from_string("conv-123".to_string());
        let cli_path = PathBuf::from("/usr/bin/claude");
        let plugin_dir = PathBuf::from("/plugins");
        let wd = PathBuf::from("/tmp");

        trigger_memory_pipelines(
            ChatContextType::TaskExecution,
            "task-123",
            &conv_id,
            None, // No project ID
            None,
            &cli_path,
            &plugin_dir,
            &wd,
            None,
        )
        .await;
        // Test passes if no panic
    }

    #[tokio::test]
    async fn test_trigger_memory_pipelines_recursion_guard_maintainer() {
        // Should return early when agent is memory-maintainer
        let project_id = ProjectId::from_string("proj-123".to_string());
        let conv_id = ChatConversationId::from_string("conv-123".to_string());
        let cli_path = PathBuf::from("/usr/bin/claude");
        let plugin_dir = PathBuf::from("/plugins");
        let wd = PathBuf::from("/tmp");

        trigger_memory_pipelines(
            ChatContextType::TaskExecution,
            "task-123",
            &conv_id,
            Some(&project_id),
            Some("memory-maintainer"), // Recursion guard
            &cli_path,
            &plugin_dir,
            &wd,
            None,
        )
        .await;
        // Test passes if no spawn happens (verified via logs in real scenario)
    }

    #[tokio::test]
    async fn test_trigger_memory_pipelines_recursion_guard_capture() {
        // Should return early when agent is memory-capture
        let project_id = ProjectId::from_string("proj-123".to_string());
        let conv_id = ChatConversationId::from_string("conv-123".to_string());
        let cli_path = PathBuf::from("/usr/bin/claude");
        let plugin_dir = PathBuf::from("/plugins");
        let wd = PathBuf::from("/tmp");

        trigger_memory_pipelines(
            ChatContextType::TaskExecution,
            "task-123",
            &conv_id,
            Some(&project_id),
            Some("memory-capture"), // Recursion guard
            &cli_path,
            &plugin_dir,
            &wd,
            None,
        )
        .await;
        // Test passes if no spawn happens
    }

    #[tokio::test]
    async fn test_spawn_memory_maintainer_fails_in_test_env() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let conv_id = ChatConversationId::from_string("conv-123".to_string());
        let cli_path = PathBuf::from("/usr/bin/claude");
        let plugin_dir = PathBuf::from("/plugins");
        let wd = PathBuf::from("/tmp");

        let result = spawn_memory_maintainer(
            &conv_id,
            ChatContextType::TaskExecution,
            "task-123",
            &project_id,
            &cli_path,
            &plugin_dir,
            &wd,
        )
        .await;

        // In test environment, build_spawnable_command returns Err due to ensure_claude_spawn_allowed()
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spawn_memory_capture_fails_in_test_env() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let conv_id = ChatConversationId::from_string("conv-123".to_string());
        let cli_path = PathBuf::from("/usr/bin/claude");
        let plugin_dir = PathBuf::from("/plugins");
        let wd = PathBuf::from("/tmp");

        let result = spawn_memory_capture(
            &conv_id,
            ChatContextType::TaskExecution,
            "task-123",
            &project_id,
            &cli_path,
            &plugin_dir,
            &wd,
        )
        .await;

        // In test environment, build_spawnable_command returns Err due to ensure_claude_spawn_allowed()
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_pipelines_parallel_spawn_both_enabled() {
        // "execution" is in both maintenance_categories AND capture_categories by default
        let project_id = ProjectId::from_string("proj-123".to_string());
        let settings = ProjectMemorySettings::default();

        let result = resolve_pipelines(
            ChatContextType::TaskExecution,
            Some(&project_id),
            Some("ralphx:ralphx-worker"),
            &settings,
        );

        assert!(result.is_some(), "Should return Some when category is enabled");
        let (should_maintain, should_capture) = result.unwrap();
        assert!(should_maintain, "execution should be in maintenance_categories");
        assert!(should_capture, "execution should be in capture_categories");
    }

    #[test]
    fn test_resolve_pipelines_disabled_project_skips_spawn() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let settings = ProjectMemorySettings {
            enabled: false,
            ..ProjectMemorySettings::default()
        };

        let result = resolve_pipelines(
            ChatContextType::TaskExecution,
            Some(&project_id),
            Some("ralphx:ralphx-worker"),
            &settings,
        );

        assert!(result.is_none(), "Should return None when memory is disabled");
    }
}
