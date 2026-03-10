//! Helper functions for HTTP server handlers
//!
//! Extracted from http_server.rs to manage file size and maintain separation of concerns.
//! Contains parsing, transformation, and context aggregation functions.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::application::{AppState, CreateProposalOptions, UpdateProposalOptions, UpdateSource};
use crate::domain::services::{check_proposal_verification_gate, ProposalOperation};
use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
use crate::commands::ideation_commands::TaskProposalResponse;
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactSummary, ArtifactType, Complexity, IdeationSession,
    IdeationSessionId, IdeationSessionStatus, InternalStatus, Priority, ProposalCategory,
    TaskContext, TaskId, TaskProposal, TaskProposalId,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::{
    SqliteArtifactRepository as ArtifactRepo, SqliteIdeationSessionRepository as SessionRepo,
    SqliteTaskProposalRepository as ProposalRepo,
};
use tauri::Emitter;

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse a category string to ProposalCategory enum
///
/// Accepts: "feature", "fix"/"bug", "refactor", "test"/"testing",
/// "docs"/"documentation", "setup"/"infrastructure"/"infra",
/// "performance"/"perf", "security"/"sec", "devops"/"dev_ops"/"ci_cd"/"cicd",
/// "research"/"investigation", "design", "chore"/"maintenance"
pub fn parse_category(s: &str) -> Result<ProposalCategory, String> {
    match s.to_lowercase().as_str() {
        "feature" => Ok(ProposalCategory::Feature),
        "fix" | "bug" => Ok(ProposalCategory::Fix),
        "refactor" => Ok(ProposalCategory::Refactor),
        "test" | "testing" => Ok(ProposalCategory::Test),
        "docs" | "documentation" => Ok(ProposalCategory::Docs),
        "setup" | "infrastructure" | "infra" => Ok(ProposalCategory::Setup),
        "performance" | "perf" => Ok(ProposalCategory::Performance),
        "security" | "sec" => Ok(ProposalCategory::Security),
        "devops" | "dev_ops" | "ci_cd" | "cicd" => Ok(ProposalCategory::DevOps),
        "research" | "investigation" => Ok(ProposalCategory::Research),
        "design" => Ok(ProposalCategory::Design),
        "chore" | "maintenance" => Ok(ProposalCategory::Chore),
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

/// Create proposal — all checks and INSERT in a single DB transaction.
///
/// Session existence, active status, plan artifact requirement, and sort_order count
/// are verified inside `db.run_transaction()` to prevent TOCTOU races. Events and
/// dependency analysis are emitted after the transaction returns.
///
/// # Errors
/// - `AppError::NotFound` if session or plan artifact doesn't exist
/// - `AppError::Validation` if session is not active or has no plan artifact
/// - Database errors from the proposal repository
pub async fn create_proposal_impl(
    state: &AppState,
    session_id: IdeationSessionId,
    options: CreateProposalOptions,
) -> AppResult<TaskProposal> {
    // Single lock: all checks + INSERT in one transaction (TOCTOU prevention).
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let proposal = state
        .db
        .run_transaction(move |conn| {
            // Check session exists and is active
            let session = SessionRepo::get_by_id_sync(conn, session_id.as_str())?
                .ok_or_else(|| AppError::NotFound(format!("Session {} not found", session_id)))?;

            if session.status != IdeationSessionStatus::Active {
                return Err(AppError::Validation(format!(
                    "Cannot add proposal to {} session",
                    session.status
                )));
            }

            // Verification gate: block creation if plan hasn't been verified (when enabled)
            {
                let settings = get_settings_sync(conn)?;
                let parent_status = if session.plan_artifact_id.is_none()
                    && session.inherited_plan_artifact_id.is_some()
                {
                    session
                        .parent_session_id
                        .as_ref()
                        .and_then(|pid| {
                            SessionRepo::get_by_id_sync(conn, pid.as_str())
                                .ok()
                                .flatten()
                        })
                        .map(|p| p.verification_status)
                } else {
                    None
                };
                check_proposal_verification_gate(
                    &session,
                    &settings,
                    parent_status,
                    ProposalOperation::Create,
                )
                .map_err(AppError::from)?;
            }

            // Enforce plan artifact requirement
            let plan_artifact_id = session.plan_artifact_id.ok_or_else(|| {
                AppError::Validation(
                    "Proposals can only be created when a plan artifact exists for this session. \
                     Use create_plan_artifact first."
                        .to_string(),
                )
            })?;

            // Fetch artifact version for auto-linking
            let artifact = ArtifactRepo::get_by_id_sync(conn, plan_artifact_id.as_str())?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Plan artifact {} not found", plan_artifact_id))
                })?;

            // Count proposals for sort_order (within same lock — no TOCTOU)
            let count = ProposalRepo::count_by_session_sync(conn, session_id.as_str())?;

            // Build proposal with auto-linked plan artifact
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
            proposal.plan_version_at_creation = Some(artifact.metadata.version);
            proposal.plan_artifact_id = Some(plan_artifact_id);
            if let Some(complexity_str) = options.estimated_complexity {
                if let Ok(c) = complexity_str.parse::<Complexity>() {
                    proposal.estimated_complexity = c;
                }
            }

            ProposalRepo::create_sync(conn, proposal)
        })
        .await?;

    // Emit event after transaction (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_handle {
        let response = TaskProposalResponse::from(proposal.clone());
        let _ = app_handle.emit(
            "proposal:created",
            serde_json::json!({ "proposal": response }),
        );
    }

    // Auto-trigger dependency analysis when we have 2+ proposals
    maybe_trigger_dependency_analysis(&proposal.session_id, state).await;

    Ok(proposal)
}

/// Update proposal — fetch, validate, and UPDATE in a single DB transaction.
///
/// `assert_session_mutable()` is called inside the transaction (bug fix: IPC update
/// path was previously missing this guard). When `options.source == TauriIpc`, sets
/// `user_modified = true` per changed field and calls `proposal.touch()`. Events and
/// dependency analysis are emitted after the transaction returns.
///
/// # Errors
/// - `AppError::NotFound` if proposal or session doesn't exist
/// - `AppError::Validation` if session is Archived or Accepted
/// - Database errors from the proposal repository
pub async fn update_proposal_impl(
    state: &AppState,
    proposal_id: &TaskProposalId,
    options: UpdateProposalOptions,
) -> AppResult<TaskProposal> {
    let pid = proposal_id.as_str().to_string();

    // Single lock: fetch + validate + UPDATE in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let updated = state
        .db
        .run_transaction(move |conn| {
            // Fetch proposal
            let mut proposal = conn
                .query_row(
                    "SELECT id, session_id, title, description, category, steps, acceptance_criteria,
                            suggested_priority, priority_score, priority_reason, priority_factors,
                            estimated_complexity, user_priority, user_modified, status, selected,
                            created_task_id, plan_artifact_id, plan_version_at_creation, sort_order, created_at, updated_at
                     FROM task_proposals WHERE id = ?1",
                    [&pid],
                    |row| TaskProposal::from_row(row),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Proposal {} not found", pid))
                    }
                    other => AppError::from(other),
                })?;

            // Guard: reject mutations on Archived/Accepted sessions (bug fix: IPC update was ungated)
            let session =
                SessionRepo::get_by_id_sync(conn, proposal.session_id.as_str())?.ok_or_else(
                    || AppError::NotFound(format!("Session {} not found", proposal.session_id)),
                )?;
            assert_session_mutable(&session)?;

            // Verification gate: block update if verification in progress or needs revision
            {
                let settings = get_settings_sync(conn)?;
                let parent_status = if session.plan_artifact_id.is_none()
                    && session.inherited_plan_artifact_id.is_some()
                {
                    session
                        .parent_session_id
                        .as_ref()
                        .and_then(|pid| {
                            SessionRepo::get_by_id_sync(conn, pid.as_str())
                                .ok()
                                .flatten()
                        })
                        .map(|p| p.verification_status)
                } else {
                    None
                };
                check_proposal_verification_gate(
                    &session,
                    &settings,
                    parent_status,
                    ProposalOperation::Update,
                )
                .map_err(AppError::from)?;
            }

            let is_ipc = matches!(options.source, UpdateSource::TauriIpc);

            // Apply updates; track user_modified per field when source is TauriIpc
            if let Some(title) = options.title {
                proposal.title = title;
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(description) = options.description {
                proposal.description = description;
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(category) = options.category {
                proposal.category = category;
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(steps) = options.steps {
                proposal.steps = steps;
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(acceptance_criteria) = options.acceptance_criteria {
                proposal.acceptance_criteria = acceptance_criteria;
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(priority) = options.user_priority {
                proposal.user_priority = Some(priority);
                if is_ipc {
                    proposal.user_modified = true;
                }
            }
            if let Some(complexity_str) = options.estimated_complexity {
                if let Ok(complexity) = complexity_str.parse::<Complexity>() {
                    proposal.estimated_complexity = complexity;
                    if is_ipc {
                        proposal.user_modified = true;
                    }
                }
            }

            // Touch timestamp when user-originated (matches IPC command behaviour)
            if is_ipc {
                proposal.touch();
            }

            ProposalRepo::update_sync(conn, &proposal)
        })
        .await?;

    // Emit event after transaction (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_handle {
        let response = TaskProposalResponse::from(updated.clone());
        let _ = app_handle.emit(
            "proposal:updated",
            serde_json::json!({ "proposal": response }),
        );
    }

    // Auto-trigger dependency analysis when proposals change
    maybe_trigger_dependency_analysis(&updated.session_id, state).await;

    Ok(updated)
}

/// Delete proposal — fetch session, assert mutability, and DELETE in a single DB transaction.
///
/// Fixes existing bug: HTTP delete handler had no `assert_session_mutable()` guard, allowing
/// MCP agents to delete proposals from Archived/Accepted sessions.
///
/// # Errors
/// - `AppError::NotFound` if proposal or session doesn't exist
/// - `AppError::Validation` if session is Archived or Accepted
/// - Database errors from the proposal repository
pub async fn delete_proposal_impl(
    state: &AppState,
    proposal_id: TaskProposalId,
) -> AppResult<IdeationSessionId> {
    let pid = proposal_id.as_str().to_string();

    // Single lock: fetch proposal+session, assert mutability, DELETE — all in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let session_id = state
        .db
        .run_transaction(move |conn| {
            // Fetch session_id from proposal
            let session_id_str: String = match conn.query_row(
                "SELECT session_id FROM task_proposals WHERE id = ?1",
                [&pid],
                |row| row.get(0),
            ) {
                Ok(s) => s,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(AppError::NotFound(format!("Proposal {} not found", pid)));
                }
                Err(e) => return Err(AppError::from(e)),
            };

            let session_id = IdeationSessionId::from_string(session_id_str);

            // Guard: reject mutations on Archived/Accepted sessions (bug fix: HTTP delete was ungated)
            let session =
                SessionRepo::get_by_id_sync(conn, session_id.as_str())?.ok_or_else(|| {
                    AppError::NotFound(format!("Session {} not found", session_id))
                })?;
            assert_session_mutable(&session)?;

            // Verification gate: block delete if verification in progress or needs revision
            {
                let settings = get_settings_sync(conn)?;
                let parent_status = if session.plan_artifact_id.is_none()
                    && session.inherited_plan_artifact_id.is_some()
                {
                    session
                        .parent_session_id
                        .as_ref()
                        .and_then(|pid| {
                            SessionRepo::get_by_id_sync(conn, pid.as_str())
                                .ok()
                                .flatten()
                        })
                        .map(|p| p.verification_status)
                } else {
                    None
                };
                check_proposal_verification_gate(
                    &session,
                    &settings,
                    parent_status,
                    ProposalOperation::Delete,
                )
                .map_err(AppError::from)?;
            }

            // Delete proposal scoped to session (prevents cross-session deletions)
            ProposalRepo::delete_sync(conn, &pid, session_id.as_str())?;

            Ok(session_id)
        })
        .await?;

    // Emit event after transaction (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_handle {
        let _ = app_handle.emit(
            "proposal:deleted",
            serde_json::json!({ "proposalId": proposal_id.as_str() }),
        );
    }

    // Auto-trigger dependency analysis after deletion (if we still have 2+ proposals)
    maybe_trigger_dependency_analysis(&session_id, state).await;

    Ok(session_id)
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

    // 7. Fetch task dependencies (blockers and dependents) via TaskDependencyRepository
    let blocker_ids = state.task_dependency_repo.get_blockers(task_id).await?;
    let mut blocked_by: Vec<crate::domain::entities::TaskDependencySummary> = Vec::new();
    for blocker_id in &blocker_ids {
        if let Some(blocker_task) = state.task_repo.get_by_id(blocker_id).await? {
            blocked_by.push(crate::domain::entities::TaskDependencySummary {
                id: blocker_task.id.clone(),
                title: blocker_task.title.clone(),
                internal_status: blocker_task.internal_status,
            });
        }
    }

    let dependent_ids = state.task_dependency_repo.get_blocked_by(task_id).await?;
    let mut blocks: Vec<crate::domain::entities::TaskDependencySummary> = Vec::new();
    for dep_id in &dependent_ids {
        if let Some(dep_task) = state.task_repo.get_by_id(dep_id).await? {
            blocks.push(crate::domain::entities::TaskDependencySummary {
                id: dep_task.id.clone(),
                title: dep_task.title.clone(),
                internal_status: dep_task.internal_status,
            });
        }
    }

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
    let analyzing_dependencies = Arc::clone(&app_state.analyzing_dependencies);
    let debounce_generations = Arc::clone(&app_state.debounce_generations);

    // Increment generation counter and capture value before spawning.
    // Use a scoped block to ensure the std::sync::Mutex is released before any .await point.
    let captured_gen = {
        let mut gens = debounce_generations.lock().unwrap();
        let gen = gens
            .entry(IdeationSessionId::from_string(session_id_str.clone()))
            .or_insert(0);
        *gen = gen.wrapping_add(1);
        *gen
    };

    // Spawn with debounce delay
    tokio::spawn(async move {
        // Debounce: wait 2 seconds before triggering
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Gate 1: gen staleness check — if a newer trigger arrived, discard this task
        {
            let gens = debounce_generations.lock().unwrap();
            let current_gen = gens
                .get(&IdeationSessionId::from_string(session_id_str.clone()))
                .copied()
                .unwrap_or(0);
            if current_gen != captured_gen {
                tracing::debug!(
                    "Skipping stale dependency analysis trigger for session {}, gen {} != {}",
                    session_id_str,
                    captured_gen,
                    current_gen
                );
                return;
            }
        }

        // Gate 2: analysis already in progress — skip if another agent is running
        {
            let analyzing = analyzing_dependencies.read().await;
            if analyzing.contains(&IdeationSessionId::from_string(session_id_str.clone())) {
                tracing::debug!(
                    "Skipping dependency analysis for session {}, analysis already in progress",
                    session_id_str
                );
                return;
            }
        }

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

        // Mark session as analyzing before emitting the started event
        {
            let mut analyzing = analyzing_dependencies.write().await;
            analyzing.insert(IdeationSessionId::from_string(session_id_str.clone()));
        }

        // Spawn safety timeout: auto-clear stale entry after 60 seconds
        let timeout_analyzing = Arc::clone(&analyzing_dependencies);
        let timeout_session_id = session_id_str.clone();
        let timeout_app_handle = app_handle.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            let removed = {
                let mut analyzing = timeout_analyzing.write().await;
                analyzing.remove(&IdeationSessionId::from_string(timeout_session_id.clone()))
            };
            if removed {
                let _ = timeout_app_handle.emit(
                    "dependencies:analysis_failed",
                    serde_json::json!({
                        "sessionId": timeout_session_id,
                        "error": "Dependency analysis timed out after 60 seconds",
                    }),
                );
            }
        });

        // Fetch plan artifact summary for the session
        let plan_summary =
            fetch_plan_summary_for_session(&session_id_str, &ideation_session_repo, &artifact_repo)
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
                let removed = {
                    let mut analyzing = analyzing_dependencies.write().await;
                    analyzing.remove(&IdeationSessionId::from_string(session_id_str.clone()))
                };
                if removed {
                    let _ = app_handle.emit(
                        "dependencies:analysis_failed",
                        serde_json::json!({
                            "sessionId": session_id_str,
                            "error": "Claude CLI not found",
                        }),
                    );
                }
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
                let removed = {
                    let mut analyzing = analyzing_dependencies.write().await;
                    analyzing.remove(&IdeationSessionId::from_string(session_id_str.clone()))
                };
                if removed {
                    let _ = app_handle.emit(
                        "dependencies:analysis_failed",
                        serde_json::json!({
                            "sessionId": session_id_str,
                            "error": format!("Spawn blocked: {}", err),
                        }),
                    );
                }
                return;
            }
        };

        // Spawn the agent
        match spawnable.spawn().await {
            Ok(mut child) => {
                // Wait for completion (fire-and-forget style, but log errors)
                let inner_analyzing = Arc::clone(&analyzing_dependencies);
                let inner_session_id = session_id_str.clone();
                let inner_app_handle = app_handle.clone();
                tokio::spawn(async move {
                    match child.wait().await {
                        Ok(status) => {
                            if !status.success() {
                                tracing::warn!(
                                    "Dependency suggester agent exited with status: {}",
                                    status
                                );
                                let removed = {
                                    let mut analyzing = inner_analyzing.write().await;
                                    analyzing.remove(&IdeationSessionId::from_string(
                                        inner_session_id.clone(),
                                    ))
                                };
                                if removed {
                                    let _ = inner_app_handle.emit(
                                        "dependencies:analysis_failed",
                                        serde_json::json!({
                                            "sessionId": inner_session_id,
                                            "error": format!("Agent exited with status: {}", status),
                                        }),
                                    );
                                }
                            }
                            // Success path: apply_proposal_dependencies handler removes from set
                        }
                        Err(e) => {
                            tracing::warn!("Failed to wait for dependency suggester agent: {}", e);
                            let removed = {
                                let mut analyzing = inner_analyzing.write().await;
                                analyzing.remove(&IdeationSessionId::from_string(
                                    inner_session_id.clone(),
                                ))
                            };
                            if removed {
                                let _ = inner_app_handle.emit(
                                    "dependencies:analysis_failed",
                                    serde_json::json!({
                                        "sessionId": inner_session_id,
                                        "error": format!("Failed to wait for agent: {}", e),
                                    }),
                                );
                            }
                        }
                    }
                });
            }
            Err(e) => {
                tracing::warn!("Failed to spawn dependency suggester agent: {}", e);
                let removed = {
                    let mut analyzing = analyzing_dependencies.write().await;
                    analyzing.remove(&IdeationSessionId::from_string(session_id_str.clone()))
                };
                if removed {
                    let _ = app_handle.emit(
                        "dependencies:analysis_failed",
                        serde_json::json!({
                            "sessionId": session_id_str,
                            "error": format!("Failed to spawn agent: {}", e),
                        }),
                    );
                }
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
        // Trim to last newline to avoid mid-line cut.
        // `rfind('\n')` returns a byte index at an ASCII character boundary, so
        // `cut[..pos]` is always a valid UTF-8 slice.
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
#[path = "helpers_tests.rs"]
mod tests;
