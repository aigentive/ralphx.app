//! Helper functions for HTTP server handlers
//!
//! Extracted from http_server.rs to manage file size and maintain separation of concerns.
//! Contains parsing, transformation, and context aggregation functions.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::application::{AppState, CreateProposalOptions, UpdateProposalOptions};
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactSummary, ArtifactType, IdeationSession, IdeationSessionId,
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
/// "docs"/"documentation", "setup"/"infrastructure"/"infra",
/// "performance"/"perf", "security"/"sec", "devops"/"dev_ops"/"ci_cd"/"cicd",
/// "research"/"investigation", "design", "chore"/"maintenance"
pub fn parse_category(s: &str) -> Result<TaskCategory, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(TaskCategory::Feature),
        "fix" | "bug" => Ok(TaskCategory::Fix),
        "refactor" => Ok(TaskCategory::Refactor),
        "test" | "testing" => Ok(TaskCategory::Test),
        "docs" | "documentation" => Ok(TaskCategory::Docs),
        "setup" | "infrastructure" | "infra" => Ok(TaskCategory::Setup),
        "performance" | "perf" => Ok(TaskCategory::Performance),
        "security" | "sec" => Ok(TaskCategory::Security),
        "devops" | "dev_ops" | "ci_cd" | "cicd" => Ok(TaskCategory::DevOps),
        "research" | "investigation" => Ok(TaskCategory::Research),
        "design" => Ok(TaskCategory::Design),
        "chore" | "maintenance" => Ok(TaskCategory::Chore),
        _ => Err(format!(
            "Invalid category: '{}'. Valid: setup, feature, fix, refactor, docs, test, performance, security, devops, research, design, chore",
            s
        )),
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

    if full_content.chars().count() <= 500 {
        full_content
    } else {
        let truncated: String = full_content.chars().take(500).collect();
        format!("{truncated}...")
    }
}

// ============================================================================
// Session Guard
// ============================================================================

/// Assert that a session can be mutated (not Archived or Accepted).
///
/// Returns `Ok(())` for Active sessions.
/// Returns `AppError::Validation` for Archived/Accepted sessions, preventing
/// silent mutation of immutable sessions.
///
/// # Reference pattern
/// `create_task_proposal` (Tauri IPC) is the original protected handler.
pub fn assert_session_mutable(session: &IdeationSession) -> AppResult<()> {
    match session.status {
        IdeationSessionStatus::Archived | IdeationSessionStatus::Accepted => {
            Err(AppError::Validation(format!(
                "Cannot modify {} session. Reopen it first.",
                session.status
            )))
        }
        IdeationSessionStatus::Active => Ok(()),
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
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Session {} not found", session_id)))?;

    if session.status != IdeationSessionStatus::Active {
        return Err(AppError::Validation(format!(
            "Cannot add proposal to {} session",
            session.status
        )));
    }

    // Enforce plan artifact requirement
    let plan_artifact_id = session.plan_artifact_id.as_ref().ok_or_else(|| {
        AppError::Validation(
            "Proposals can only be created when a plan artifact exists for this session. \
             Use create_plan_artifact first."
                .to_string(),
        )
    })?;

    // Fetch current artifact version for auto-linking
    let artifact = state
        .artifact_repo
        .get_by_id(plan_artifact_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Plan artifact {} not found", plan_artifact_id))
        })?;

    // Get current proposal count for sort_order
    let count = state
        .task_proposal_repo
        .count_by_session(&session_id)
        .await?;

    // Create proposal with auto-linked plan artifact
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
    proposal.plan_artifact_id = Some(plan_artifact_id.clone());
    proposal.plan_version_at_creation = Some(artifact.metadata.version);

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
        .ok_or_else(|| AppError::NotFound(format!("Proposal {} not found", proposal_id)))?;

    // Guard: reject mutations on Archived/Accepted sessions
    let session = state
        .ideation_session_repo
        .get_by_id(&proposal.session_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Session {} not found", proposal.session_id))
        })?;
    assert_session_mutable(&session)?;

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
pub async fn get_task_context_impl(state: &AppState, task_id: &TaskId) -> AppResult<TaskContext> {
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
                    priority_score: proposal.priority_score,
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
        Some(crate::domain::entities::StepProgressSummary::from_steps(
            task_id, &steps,
        ))
    } else {
        None
    };

    // 7. Fetch task dependencies (blockers and dependents)
    let blockers = state.task_repo.get_blockers(task_id).await?;
    let blocked_by: Vec<crate::domain::entities::TaskDependencySummary> = blockers
        .into_iter()
        .map(|t| crate::domain::entities::TaskDependencySummary {
            id: t.id.clone(),
            title: t.title.clone(),
            internal_status: t.internal_status,
        })
        .collect();

    let dependents = state.task_repo.get_dependents(task_id).await?;
    let blocks: Vec<crate::domain::entities::TaskDependencySummary> = dependents
        .into_iter()
        .map(|t| crate::domain::entities::TaskDependencySummary {
            id: t.id.clone(),
            title: t.title.clone(),
            internal_status: t.internal_status,
        })
        .collect();

    // 8. Compute tier from dependency depth
    let tier = if blocked_by.is_empty() {
        Some(1)
    } else {
        let incomplete_blockers = blocked_by
            .iter()
            .filter(|b| {
                !matches!(
                    b.internal_status,
                    crate::domain::entities::InternalStatus::Approved
                )
            })
            .count();
        Some((incomplete_blockers as u32) + 1)
    };

    // 9. Generate context hints
    let mut context_hints = Vec::new();

    // CRITICAL: Dependency hints come first
    if !blocked_by.is_empty() {
        let incomplete: Vec<_> = blocked_by
            .iter()
            .filter(|b| {
                !matches!(
                    b.internal_status,
                    crate::domain::entities::InternalStatus::Approved
                )
            })
            .collect();
        if !incomplete.is_empty() {
            let names: Vec<_> = incomplete.iter().map(|t| t.title.as_str()).collect();
            context_hints.push(format!(
                "BLOCKED: Task cannot proceed - waiting for: {}",
                names.join(", ")
            ));
        } else {
            context_hints.push("All blocking tasks completed - ready to execute".to_string());
        }
    }

    if !blocks.is_empty() {
        let names: Vec<_> = blocks.iter().map(|t| t.title.as_str()).collect();
        context_hints.push(format!(
            "Downstream impact: completing this task unblocks: {}",
            names.join(", ")
        ));
    }

    // CRITICAL: Branch safety hint — agents must stay on their assigned branch
    if let Some(ref branch) = task.task_branch {
        context_hints.push(format!(
            "GIT BRANCH: You are on branch '{}'. Do NOT checkout other branches (especially main/master). All work must stay on this branch.",
            branch
        ));
    }

    if source_proposal.is_some() {
        context_hints.push(
            "Task was created from ideation proposal - check acceptance criteria".to_string(),
        );
    }
    if plan_artifact.is_some() {
        context_hints.push(
            "Implementation plan available - use get_artifact to read, then extract ONLY the section relevant to YOUR task"
                .to_string(),
        );
        context_hints.push(format!(
            "SCOPE: The plan contains multiple tasks. Execute ONLY work for: \"{}\". Other tasks have their own workers.",
            task.title
        ));
    }
    if !related_artifacts.is_empty() {
        context_hints.push(format!(
            "{} related artifact{} found - may contain useful context",
            related_artifacts.len(),
            if related_artifacts.len() == 1 {
                ""
            } else {
                "s"
            }
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

    // Surface restart_note from task metadata (one-shot, cleared after agent reads in on_enter_states)
    if let Some(ref metadata_str) = task.metadata {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(metadata_str) {
            if let Some(note) = meta.get("restart_note").and_then(|v| v.as_str()) {
                context_hints.push(format!("RESTART NOTE from user: {}", note));
            }
        }
    }

    if context_hints.is_empty() {
        context_hints.push("No additional context artifacts found - proceed with task description and acceptance criteria".to_string());
    }

    // 10. Return TaskContext
    let task_branch = task.task_branch.clone();
    let worktree_path = task.worktree_path.clone();
    Ok(TaskContext {
        task,
        source_proposal,
        plan_artifact,
        related_artifacts,
        steps,
        step_progress,
        context_hints,
        blocked_by,
        blocks,
        tier,
        task_branch,
        worktree_path,
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
    let count = match app_state
        .task_proposal_repo
        .get_by_session(session_id)
        .await
    {
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
    let ideation_session_repo = Arc::clone(&app_state.ideation_session_repo);
    let artifact_repo = Arc::clone(&app_state.artifact_repo);

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

        // Fetch plan artifact summary for the session
        let plan_summary = fetch_plan_summary_for_session(
            &session_id_str,
            &ideation_session_repo,
            &artifact_repo,
        )
        .await;

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

        // Build existing dependencies summary with source labels when available
        let existing_deps_summary = if existing_deps.is_empty() {
            "None".to_string()
        } else {
            existing_deps
                .iter()
                .map(|(from, to, _reason)| format!("{} → {}", from.as_str(), to.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Build plan summary section (injected before the analysis instruction)
        let plan_section = if plan_summary.is_empty() {
            String::new()
        } else {
            format!("\nImplementation Plan Summary:\n{}\n", plan_summary)
        };

        // Build the prompt
        let prompt = format!(
            "Session ID: {}\n\nProposals:\n{}\nExisting dependencies: {}\n{}
Analyze these proposals and identify logical dependencies based on their content. Call the apply_proposal_dependencies tool with your findings.",
            session_id_str, proposal_summaries, existing_deps_summary, plan_section
        );

        // Emit analysis started event
        let _ = app_handle.emit(
            "dependencies:analysis_started",
            serde_json::json!({
                "sessionId": session_id_str,
            }),
        );

        // Find working directory (project root where ralphx-plugin exists)
        let working_directory = find_project_root();
        let plugin_dir =
            crate::infrastructure::agents::claude::resolve_plugin_dir(&working_directory);

        // Find Claude CLI
        let cli_path = match crate::infrastructure::agents::claude::find_claude_cli() {
            Some(path) => path,
            None => {
                tracing::warn!("Failed to spawn dependency suggester: Claude CLI not found");
                return;
            }
        };

        // Build spawnable command using the established pattern
        let agent_name =
            crate::infrastructure::agents::claude::agent_names::AGENT_DEPENDENCY_SUGGESTER;
        let spawnable = match crate::infrastructure::agents::claude::build_spawnable_command(
            &cli_path,
            &plugin_dir,
            &prompt,
            Some(agent_name),
            None, // No resume session
            &working_directory,
        ) {
            Ok(cmd) => cmd,
            Err(err) => {
                tracing::warn!("Dependency suggester spawn blocked: {}", err);
                return;
            }
        };

        // Spawn the agent
        match spawnable.spawn().await {
            Ok(mut child) => {
                // Wait for completion (fire-and-forget style, but log errors)
                tokio::spawn(async move {
                    match child.wait().await {
                        Ok(status) => {
                            if !status.success() {
                                tracing::warn!(
                                    "Dependency suggester agent exited with status: {}",
                                    status
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to wait for dependency suggester agent: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                tracing::warn!("Failed to spawn dependency suggester agent: {}", e);
            }
        }
    });
}

// ============================================================================
// Plan Summarization for Dependency Analysis
// ============================================================================

/// Extract a structured summary of plan phases and ordering notes from markdown text.
///
/// Scans the text for markdown headings (`## Phase N`, `### ...`) and numbered/bullet
/// ordering lines. Returns a "Plan Structure:" block truncated to ~1500 chars.
/// Returns an empty string if the input is empty.
pub fn summarize_plan_for_dependencies(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut lines_out: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Include markdown headings (## Phase N, ### subsections)
        if trimmed.starts_with("##") {
            lines_out.push(trimmed.to_string());
            continue;
        }

        // Include numbered list items (1. ..., 2. ...)
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            let after_digit: &str = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
            if after_digit.starts_with(". ") || after_digit.starts_with(") ") {
                lines_out.push(trimmed.to_string());
                continue;
            }
        }

        // Include bullet ordering notes with ordering keywords
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            let lower = content.to_lowercase();
            if lower.contains("phase")
                || lower.contains("before")
                || lower.contains("after")
                || lower.contains("order")
                || lower.contains("first")
                || lower.contains("then")
                || lower.contains("depends")
                || lower.contains("foundation")
                || lower.contains("wave")
            {
                lines_out.push(trimmed.to_string());
                continue;
            }
        }
    }

    if lines_out.is_empty() {
        return String::new();
    }

    let body = lines_out.join("\n");

    // Truncate to ~1500 chars
    const MAX_CHARS: usize = 1500;
    let truncated = if body.chars().count() > MAX_CHARS {
        let cut: String = body.chars().take(MAX_CHARS).collect();
        // Trim to last newline to avoid mid-line cut
        match cut.rfind('\n') {
            Some(pos) => cut[..pos].to_string(),
            None => cut,
        }
    } else {
        body
    };

    format!("Plan Structure:\n{}", truncated)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Fetch the plan artifact summary for a session, returning empty string if none exists.
///
/// Looks up the session by ID, retrieves its plan_artifact_id, fetches the artifact,
/// and calls `summarize_plan_for_dependencies` on the inline content.
async fn fetch_plan_summary_for_session(
    session_id_str: &str,
    ideation_session_repo: &Arc<dyn crate::domain::repositories::IdeationSessionRepository>,
    artifact_repo: &Arc<dyn crate::domain::repositories::ArtifactRepository>,
) -> String {
    let session_id = IdeationSessionId::from_string(session_id_str.to_string());

    // Fetch session to get plan_artifact_id
    let session = match ideation_session_repo.get_by_id(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::debug!("Session {} not found for plan summary", session_id_str);
            return String::new();
        }
        Err(e) => {
            tracing::warn!("Failed to fetch session for plan summary: {}", e);
            return String::new();
        }
    };

    // Extract plan artifact ID
    let artifact_id = match session.plan_artifact_id {
        Some(id) => id,
        None => return String::new(),
    };

    // Fetch artifact content
    let artifact = match artifact_repo.get_by_id(&artifact_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            tracing::debug!("Plan artifact {} not found", artifact_id);
            return String::new();
        }
        Err(e) => {
            tracing::warn!("Failed to fetch plan artifact: {}", e);
            return String::new();
        }
    };

    // Extract text from artifact content
    let text = match &artifact.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { .. } => return String::new(),
    };

    summarize_plan_for_dependencies(&text)
}

/// Find the project root directory where ralphx-plugin exists
///
/// Checks the current directory and parent for the presence of ralphx-plugin.
/// Falls back to current directory if not found.
fn find_project_root() -> PathBuf {
    std::env::current_dir()
        .map(|cwd| {
            // Check if we're in project root (ralphx-plugin exists)
            if cwd.join("ralphx-plugin").exists() {
                cwd
            // Check if we're in src-tauri (parent has ralphx-plugin)
            } else if let Some(parent) = cwd.parent() {
                if parent.join("ralphx-plugin").exists() {
                    parent.to_path_buf()
                } else {
                    cwd
                }
            } else {
                cwd
            }
        })
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{
        Artifact, ArtifactType, IdeationSession, IdeationSessionStatus, ProjectId, TaskCategory,
    };

    // -------------------------------------------------------------------------
    // assert_session_mutable tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_assert_session_mutable_active_ok() {
        let session = IdeationSession::new_with_title(ProjectId::new(), "Active Session");
        assert_eq!(session.status, IdeationSessionStatus::Active);
        assert!(assert_session_mutable(&session).is_ok());
    }

    #[test]
    fn test_assert_session_mutable_archived_err() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .title("Archived Session")
            .status(IdeationSessionStatus::Archived)
            .build();
        let result = assert_session_mutable(&session);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation(msg) => {
                assert!(msg.contains("archived"), "Expected 'archived' in: {}", msg);
                assert!(msg.contains("Reopen"), "Expected 'Reopen' in: {}", msg);
            }
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[test]
    fn test_assert_session_mutable_accepted_err() {
        let session = IdeationSession::builder()
            .project_id(ProjectId::new())
            .title("Accepted Session")
            .status(IdeationSessionStatus::Accepted)
            .build();
        let result = assert_session_mutable(&session);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation(msg) => {
                assert!(msg.contains("accepted"), "Expected 'accepted' in: {}", msg);
            }
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_proposal_without_plan_artifact_returns_validation_error() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create a session WITHOUT a plan artifact
        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
        let session_id = session.id.clone();
        state.ideation_session_repo.create(session).await.unwrap();

        let options = CreateProposalOptions {
            title: "Test Proposal".to_string(),
            description: None,
            category: TaskCategory::Feature,
            suggested_priority: Priority::Medium,
            steps: None,
            acceptance_criteria: None,
        };

        let result = create_proposal_impl(&state, session_id, options).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            AppError::Validation(msg) => {
                assert!(
                    msg.contains("plan artifact"),
                    "Error message should mention plan artifact, got: {}",
                    msg
                );
            }
            other => panic!("Expected AppError::Validation, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_proposal_with_plan_artifact_succeeds_and_auto_links() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create artifact first
        let artifact = Artifact::new_inline(
            "Test Plan",
            ArtifactType::Specification,
            "# Plan content",
            "test",
        );
        let artifact_id = artifact.id.clone();
        state.artifact_repo.create(artifact).await.unwrap();

        // Create a session WITH a plan artifact
        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Test Session")
            .plan_artifact_id(artifact_id.clone())
            .build();
        let session_id = session.id.clone();
        state.ideation_session_repo.create(session).await.unwrap();

        let options = CreateProposalOptions {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            category: TaskCategory::Feature,
            suggested_priority: Priority::High,
            steps: None,
            acceptance_criteria: None,
        };

        let result = create_proposal_impl(&state, session_id, options).await;
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());

        let proposal = result.unwrap();
        assert_eq!(
            proposal.plan_artifact_id,
            Some(artifact_id),
            "Proposal should have plan_artifact_id auto-set from session"
        );
    }

    #[tokio::test]
    async fn test_create_proposal_sets_plan_version_at_creation() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create artifact with version 1 (default)
        let artifact = Artifact::new_inline(
            "Test Plan",
            ArtifactType::Specification,
            "# Plan v1",
            "test",
        );
        let artifact_id = artifact.id.clone();
        state.artifact_repo.create(artifact).await.unwrap();

        // Create session with plan artifact
        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Test Session")
            .plan_artifact_id(artifact_id.clone())
            .build();
        let session_id = session.id.clone();
        state.ideation_session_repo.create(session).await.unwrap();

        let options = CreateProposalOptions {
            title: "Versioned Proposal".to_string(),
            description: None,
            category: TaskCategory::Feature,
            suggested_priority: Priority::Medium,
            steps: None,
            acceptance_criteria: None,
        };

        let proposal = create_proposal_impl(&state, session_id, options)
            .await
            .unwrap();

        assert_eq!(
            proposal.plan_version_at_creation,
            Some(1),
            "Proposal should have plan_version_at_creation set to artifact's current version"
        );
    }

    // -------------------------------------------------------------------------
    // summarize_plan_for_dependencies tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_summarize_empty_input_returns_empty() {
        let result = summarize_plan_for_dependencies("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_summarize_extracts_phase_headings() {
        let input = "# Title\n\n## Phase 1: Setup\nSome prose.\n\n## Phase 2: Features\nMore prose.";
        let result = summarize_plan_for_dependencies(input);
        assert!(result.contains("## Phase 1: Setup"), "Should include phase 1 heading");
        assert!(result.contains("## Phase 2: Features"), "Should include phase 2 heading");
        assert!(result.starts_with("Plan Structure:"));
    }

    #[test]
    fn test_summarize_extracts_numbered_items() {
        let input = "## Overview\n1. First step\n2. Second step\n3. Third step";
        let result = summarize_plan_for_dependencies(input);
        assert!(result.contains("1. First step"));
        assert!(result.contains("2. Second step"));
        assert!(result.contains("3. Third step"));
    }

    #[test]
    fn test_summarize_includes_ordering_bullets() {
        let input = "## Notes\n- This task depends on setup\n- Run after the database phase\n- Unrelated bullet point";
        let result = summarize_plan_for_dependencies(input);
        assert!(result.contains("- This task depends on setup"));
        assert!(result.contains("- Run after the database phase"));
        // Unrelated bullet without ordering keywords should be excluded
        assert!(!result.contains("Unrelated bullet point"));
    }

    #[test]
    fn test_summarize_truncates_to_1500_chars() {
        // Build a long input with many headings
        let long_input: String = (1..=100)
            .map(|i| format!("## Phase {}: Some very long phase title with lots of words here\n", i))
            .collect();
        let result = summarize_plan_for_dependencies(&long_input);
        // Result (including "Plan Structure:\n" prefix) should be bounded
        // 1500 chars of body + "Plan Structure:\n" prefix (16 chars) = ~1516 max
        assert!(result.len() <= 1520, "Result should be truncated, got {} chars", result.len());
        assert!(result.starts_with("Plan Structure:"));
    }

    #[test]
    fn test_summarize_no_matching_content_returns_empty() {
        let input = "Just regular prose with no headings or numbered lists.\nAnother line of prose.";
        let result = summarize_plan_for_dependencies(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_summarize_h1_heading_excluded_h2_included() {
        let input = "# Main Title (excluded)\n## Phase 1 (included)\n### Sub-section (included)";
        let result = summarize_plan_for_dependencies(input);
        assert!(!result.contains("# Main Title"), "H1 headings should not be included");
        assert!(result.contains("## Phase 1"));
        assert!(result.contains("### Sub-section"));
    }

    // -------------------------------------------------------------------------
    // fetch_plan_summary_for_session tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_fetch_plan_summary_session_not_found_returns_empty() {
        let state = AppState::new_test();
        let result = fetch_plan_summary_for_session(
            "nonexistent-session-id",
            &state.ideation_session_repo,
            &state.artifact_repo,
        )
        .await;
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_fetch_plan_summary_session_without_plan_returns_empty() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Session WITHOUT a plan artifact
        let session = IdeationSession::new_with_title(project_id, "No Plan Session");
        let session_id = session.id.as_str().to_string();
        state.ideation_session_repo.create(session).await.unwrap();

        let result = fetch_plan_summary_for_session(
            &session_id,
            &state.ideation_session_repo,
            &state.artifact_repo,
        )
        .await;
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_fetch_plan_summary_with_plan_returns_summary() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        let plan_content = "## Phase 1: Setup\n1. Create schema\n2. Run migrations\n\n## Phase 2: Features\n1. Implement API";
        let artifact = Artifact::new_inline(
            "Implementation Plan",
            ArtifactType::Specification,
            plan_content,
            "test",
        );
        let artifact_id = artifact.id.clone();
        state.artifact_repo.create(artifact).await.unwrap();

        let session = IdeationSession::builder()
            .project_id(project_id)
            .title("Session With Plan")
            .plan_artifact_id(artifact_id)
            .build();
        let session_id = session.id.as_str().to_string();
        state.ideation_session_repo.create(session).await.unwrap();

        let result = fetch_plan_summary_for_session(
            &session_id,
            &state.ideation_session_repo,
            &state.artifact_repo,
        )
        .await;

        assert!(result.starts_with("Plan Structure:"), "Should start with Plan Structure:");
        assert!(result.contains("## Phase 1: Setup"));
        assert!(result.contains("## Phase 2: Features"));
    }
}
