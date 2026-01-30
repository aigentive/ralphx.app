//! Helper functions for HTTP server handlers
//!
//! Extracted from http_server.rs to manage file size and maintain separation of concerns.
//! Contains parsing, transformation, and context aggregation functions.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::application::{AppState, CreateProposalOptions, UpdateProposalOptions};
use crate::domain::agents::{AgentConfig, AgentRole};
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactSummary, ArtifactType, IdeationSessionId,
    IdeationSessionStatus, InternalStatus, Priority, TaskCategory, TaskContext, TaskId,
    TaskProposal, TaskProposalId,
};
use crate::error::{AppError, AppResult};
use tauri::Emitter;

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse a category string to TaskCategory enum
///
/// Accepts: "feature", "fix"/"bug", "refactor", "test"/"testing",
/// "docs"/"documentation", "setup"/"infrastructure"/"infra"
pub fn parse_category(s: &str) -> Result<TaskCategory, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(TaskCategory::Feature),
        "fix" | "bug" => Ok(TaskCategory::Fix),
        "refactor" => Ok(TaskCategory::Refactor),
        "test" | "testing" => Ok(TaskCategory::Test),
        "docs" | "documentation" => Ok(TaskCategory::Docs),
        "setup" | "infrastructure" | "infra" => Ok(TaskCategory::Setup),
        _ => Err(format!("Invalid category: {}", s)),
    }
}

/// Parse a priority string to Priority enum
///
/// Accepts: "critical"/"urgent", "high", "medium"/"med", "low"
pub fn parse_priority(s: &str) -> Result<Priority, String> {
    match s.to_lowercase().as_str() {
        "critical" | "urgent" => Ok(Priority::Critical),
        "high" => Ok(Priority::High),
        "medium" | "med" => Ok(Priority::Medium),
        "low" => Ok(Priority::Low),
        _ => Err(format!("Invalid priority: {}", s)),
    }
}

/// Parse an internal status string to InternalStatus enum
pub fn parse_internal_status(s: &str) -> Result<InternalStatus, String> {
    InternalStatus::from_str(s).map_err(|e| e.to_string())
}

/// Parse an artifact type string to ArtifactType enum
pub fn parse_artifact_type(s: &str) -> Result<ArtifactType, String> {
    match s.to_lowercase().as_str() {
        "prd" => Ok(ArtifactType::Prd),
        "specification" => Ok(ArtifactType::Specification),
        "research" | "researchdocument" | "research_document" => Ok(ArtifactType::ResearchDocument),
        "design" | "designdoc" | "design_doc" => Ok(ArtifactType::DesignDoc),
        "code_change" | "codechanges" => Ok(ArtifactType::CodeChange),
        "diff" => Ok(ArtifactType::Diff),
        "test_result" | "testresult" => Ok(ArtifactType::TestResult),
        "task_spec" | "taskspec" => Ok(ArtifactType::TaskSpec),
        "review_feedback" | "reviewfeedback" => Ok(ArtifactType::ReviewFeedback),
        "approval" => Ok(ArtifactType::Approval),
        "findings" => Ok(ArtifactType::Findings),
        "recommendations" => Ok(ArtifactType::Recommendations),
        "context" => Ok(ArtifactType::Context),
        "previous_work" | "previouswork" => Ok(ArtifactType::PreviousWork),
        "research_brief" | "researchbrief" => Ok(ArtifactType::ResearchBrief),
        _ => Err(format!("Invalid artifact type: {}", s)),
    }
}

// ============================================================================
// Transformation Functions
// ============================================================================

/// Create a 500-character preview of artifact content
///
/// Truncates large artifacts with "..." suffix, preserves smaller artifacts in full.
pub fn create_artifact_preview(artifact: &Artifact) -> String {
    let full_content = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { path } => {
            format!("[File artifact at: {}]", path)
        }
    };

    if full_content.len() <= 500 {
        full_content
    } else {
        format!("{}...", &full_content[..500])
    }
}

// ============================================================================
// Proposal Implementation Functions
// ============================================================================

/// Create proposal (reuses IdeationService logic)
///
/// Verifies session exists and is active, calculates sort order, and saves to database.
///
/// # Errors
/// - `AppError::NotFound` if session doesn't exist
/// - `AppError::Validation` if session is not active
/// - Database errors from the proposal repository
pub async fn create_proposal_impl(
    state: &AppState,
    session_id: crate::domain::entities::IdeationSessionId,
    options: CreateProposalOptions,
) -> AppResult<TaskProposal> {
    // Verify session exists and is active
    let session = state.ideation_session_repo.get_by_id(&session_id).await?;
    match session {
        None => {
            return Err(AppError::NotFound(format!(
                "Session {} not found",
                session_id
            )))
        }
        Some(s) if s.status != IdeationSessionStatus::Active => {
            return Err(AppError::Validation(format!(
                "Cannot add proposal to {} session",
                s.status
            )));
        }
        _ => {}
    }

    // Get current proposal count for sort_order
    let count = state.task_proposal_repo.count_by_session(&session_id).await?;

    // Create proposal
    let mut proposal = TaskProposal::new(
        session_id,
        options.title,
        options.category,
        options.suggested_priority,
    );
    proposal.description = options.description;
    proposal.steps = options.steps;
    proposal.acceptance_criteria = options.acceptance_criteria;
    proposal.sort_order = count as i32;

    // Save to database
    state.task_proposal_repo.create(proposal.clone()).await?;

    Ok(proposal)
}

/// Update proposal (reuses IdeationService logic)
///
/// Fetches existing proposal, applies updates to specified fields, and saves.
///
/// # Errors
/// - `AppError::NotFound` if proposal doesn't exist
/// - Database errors from the proposal repository
pub async fn update_proposal_impl(
    state: &AppState,
    proposal_id: &TaskProposalId,
    options: UpdateProposalOptions,
) -> AppResult<TaskProposal> {
    // Get existing proposal
    let mut proposal = state
        .task_proposal_repo
        .get_by_id(proposal_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Proposal {} not found", proposal_id))
        })?;

    // Apply updates
    if let Some(title) = options.title {
        proposal.title = title;
    }
    if let Some(description) = options.description {
        proposal.description = description;
    }
    if let Some(category) = options.category {
        proposal.category = category;
    }
    if let Some(steps) = options.steps {
        proposal.steps = steps;
    }
    if let Some(acceptance_criteria) = options.acceptance_criteria {
        proposal.acceptance_criteria = acceptance_criteria;
    }
    if let Some(priority) = options.user_priority {
        proposal.user_priority = Some(priority);
    }

    // Save updated proposal
    state.task_proposal_repo.update(&proposal).await?;

    Ok(proposal)
}

// ============================================================================
// Task Context Function
// ============================================================================

/// Get task context - implementation that manually aggregates context
///
/// Replicates the logic from TaskContextService but works with trait objects.
/// Fetches task, associated proposal, plan artifact, related artifacts, and steps.
/// Generates context hints based on available data.
///
/// # Errors
/// - `AppError::NotFound` if task doesn't exist
/// - Database errors from any repository
pub async fn get_task_context_impl(
    state: &AppState,
    task_id: &TaskId,
) -> AppResult<TaskContext> {
    // 1. Fetch task by ID
    let task = state
        .task_repo
        .get_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id)))?;

    // 2. If source_proposal_id present, fetch proposal and create TaskProposalSummary
    let source_proposal = if let Some(proposal_id) = &task.source_proposal_id {
        match state.task_proposal_repo.get_by_id(proposal_id).await? {
            Some(proposal) => {
                // Parse acceptance_criteria from JSON string to Vec<String>
                let acceptance_criteria: Vec<String> = proposal
                    .acceptance_criteria
                    .as_ref()
                    .and_then(|json_str| serde_json::from_str(json_str).ok())
                    .unwrap_or_default();

                Some(crate::domain::entities::TaskProposalSummary {
                    id: proposal.id.clone(),
                    title: proposal.title.clone(),
                    description: proposal.description.clone().unwrap_or_default(),
                    acceptance_criteria,
                    implementation_notes: None,
                    plan_version_at_creation: proposal.plan_version_at_creation,
                })
            }
            None => None,
        }
    } else {
        None
    };

    // 3. If plan_artifact_id present, fetch artifact and create ArtifactSummary
    let plan_artifact = if let Some(artifact_id) = &task.plan_artifact_id {
        match state.artifact_repo.get_by_id(artifact_id).await? {
            Some(artifact) => {
                let content_preview = create_artifact_preview(&artifact);
                Some(ArtifactSummary {
                    id: artifact.id.clone(),
                    title: artifact.name.clone(),
                    artifact_type: artifact.artifact_type,
                    current_version: artifact.metadata.version,
                    content_preview,
                })
            }
            None => None,
        }
    } else {
        None
    };

    // 4. Fetch related artifacts
    let related_artifacts = if let Some(artifact_id) = &task.plan_artifact_id {
        let related = state.artifact_repo.get_related(artifact_id).await?;
        related
            .into_iter()
            .map(|artifact| {
                let content_preview = create_artifact_preview(&artifact);
                ArtifactSummary {
                    id: artifact.id.clone(),
                    title: artifact.name.clone(),
                    artifact_type: artifact.artifact_type,
                    current_version: artifact.metadata.version,
                    content_preview,
                }
            })
            .collect()
    } else {
        vec![]
    };

    // 5. Fetch steps for the task
    let steps = state.task_step_repo.get_by_task(task_id).await?;

    // 6. Calculate step progress summary if steps exist
    let step_progress = if !steps.is_empty() {
        Some(crate::domain::entities::StepProgressSummary::from_steps(task_id, &steps))
    } else {
        None
    };

    // 7. Generate context hints
    let mut context_hints = Vec::new();
    if source_proposal.is_some() {
        context_hints.push(
            "Task was created from ideation proposal - check acceptance criteria".to_string(),
        );
    }
    if plan_artifact.is_some() {
        context_hints.push("Implementation plan available - use get_artifact to read full plan before starting".to_string());
    }
    if !related_artifacts.is_empty() {
        context_hints.push(format!(
            "{} related artifact{} found - may contain useful context",
            related_artifacts.len(),
            if related_artifacts.len() == 1 { "" } else { "s" }
        ));
    }
    if !steps.is_empty() {
        context_hints.push(format!(
            "Task has {} step{} defined - use get_task_steps to see them",
            steps.len(),
            if steps.len() == 1 { "" } else { "s" }
        ));
    }
    if task.description.is_some() {
        context_hints.push("Task has description with additional details".to_string());
    }
    if context_hints.is_empty() {
        context_hints.push("No additional context artifacts found - proceed with task description and acceptance criteria".to_string());
    }

    // 8. Return TaskContext
    Ok(TaskContext {
        task,
        source_proposal,
        plan_artifact,
        related_artifacts,
        steps,
        step_progress,
        context_hints,
    })
}

// ============================================================================
// Dependency Analysis Auto-Trigger
// ============================================================================

/// Auto-trigger dependency analysis if session has 2+ proposals
///
/// Callable from both HTTP handlers and Tauri commands.
/// Uses a 2-second debounce delay to avoid rapid re-triggers.
pub async fn maybe_trigger_dependency_analysis(
    session_id: &IdeationSessionId,
    app_state: &AppState,
) {
    // Get proposal count
    let count = match app_state.task_proposal_repo.get_by_session(session_id).await {
        Ok(proposals) => proposals.len(),
        Err(e) => {
            tracing::warn!("Failed to get proposals for auto-trigger: {}", e);
            return;
        }
    };

    // Only trigger if we have 2+ proposals
    if count < 2 {
        return;
    }

    // Get the app handle for emitting events
    let app_handle = match &app_state.app_handle {
        Some(handle) => handle.clone(),
        None => return, // No app handle (test environment)
    };

    // Clone what we need for the async spawn
    let session_id_str = session_id.as_str().to_string();
    let task_proposal_repo = Arc::clone(&app_state.task_proposal_repo);
    let proposal_dependency_repo = Arc::clone(&app_state.proposal_dependency_repo);
    let agent_client = Arc::clone(&app_state.agent_client);

    // Spawn with debounce delay
    tokio::spawn(async move {
        // Debounce: wait 2 seconds before triggering
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Re-fetch proposals after the delay (they may have changed)
        let proposals = match task_proposal_repo
            .get_by_session(&IdeationSessionId::from_string(session_id_str.clone()))
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to get proposals for dependency analysis: {}", e);
                return;
            }
        };

        // Still need 2+ proposals
        if proposals.len() < 2 {
            return;
        }

        // Get existing dependencies
        let existing_deps = match proposal_dependency_repo
            .get_all_for_session(&IdeationSessionId::from_string(session_id_str.clone()))
            .await
        {
            Ok(deps) => deps,
            Err(e) => {
                tracing::warn!("Failed to get dependencies for analysis: {}", e);
                Vec::new()
            }
        };

        // Build proposal summaries for the prompt
        let mut proposal_summaries = String::new();
        for (i, proposal) in proposals.iter().enumerate() {
            proposal_summaries.push_str(&format!(
                "{}. ID: {}\n   Title: \"{}\"\n   Category: {}\n   Description: {}\n\n",
                i + 1,
                proposal.id.as_str(),
                proposal.title,
                proposal.category,
                proposal.description.as_deref().unwrap_or("(none)")
            ));
        }

        // Build existing dependencies summary
        let existing_deps_summary = if existing_deps.is_empty() {
            "None".to_string()
        } else {
            existing_deps
                .iter()
                .map(|(from, to)| format!("{} → {}", from.as_str(), to.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Build the prompt
        let prompt = format!(
            "Session ID: {}\n\nProposals:\n{}\nExisting dependencies: {}\n\nAnalyze these proposals and identify logical dependencies based on their content. Call the apply_proposal_dependencies tool with your findings.",
            session_id_str, proposal_summaries, existing_deps_summary
        );

        // Emit analysis started event
        let _ = app_handle.emit(
            "dependencies:analysis_started",
            serde_json::json!({
                "sessionId": session_id_str,
            }),
        );

        // Get the working directory (project root)
        let working_directory = std::env::current_dir()
            .map(|cwd| cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd))
            .unwrap_or_else(|_| PathBuf::from("."));

        let plugin_dir = working_directory.join("ralphx-plugin");

        // Set RALPHX_AGENT_TYPE so MCP server grants access to apply_proposal_dependencies tool
        let mut env = std::collections::HashMap::new();
        env.insert(
            "RALPHX_AGENT_TYPE".to_string(),
            "dependency-suggester".to_string(),
        );

        let config = AgentConfig {
            role: AgentRole::Custom("dependency-suggester".to_string()),
            prompt,
            working_directory,
            plugin_dir: Some(plugin_dir),
            agent: Some("dependency-suggester".to_string()),
            model: None,
            max_tokens: None,
            timeout_secs: Some(60),
            env,
        };

        // Spawn the agent
        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Dependency suggester agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn dependency suggester agent: {}", e);
            }
        }
    });
}
