use super::*;
use crate::application::{AppState, UpdateProposalOptions, UpdateSource};
use crate::domain::entities::{
    Artifact, ArtifactId, ArtifactType, IdeationSession, IdeationSessionStatus, ProjectId,
    ProposalCategory,
};
use crate::error::AppError;

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
    let state = AppState::new_sqlite_test();
    let project_id = ProjectId::new();

    // Create a session WITHOUT a plan artifact
    let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
    let session_id = session.id.clone();
    state.ideation_session_repo.create(session).await.unwrap();

    let options = CreateProposalOptions {
        title: "Test Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
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
    let state = AppState::new_sqlite_test();
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
        category: ProposalCategory::Feature,
        suggested_priority: Priority::High,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
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
    let state = AppState::new_sqlite_test();
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
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
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
    assert!(
        result.contains("## Phase 1: Setup"),
        "Should include phase 1 heading"
    );
    assert!(
        result.contains("## Phase 2: Features"),
        "Should include phase 2 heading"
    );
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
    let long_input: String = (1..=100).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        writeln!(
            acc,
            "## Phase {}: Some very long phase title with lots of words here",
            i
        )
        .unwrap();
        acc
    });
    let result = summarize_plan_for_dependencies(&long_input);
    // Result (including "Plan Structure:\n" prefix) should be bounded
    // 1500 chars of body + "Plan Structure:\n" prefix (16 chars) = ~1516 max
    assert!(
        result.len() <= 1520,
        "Result should be truncated, got {} chars",
        result.len()
    );
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
    assert!(
        !result.contains("# Main Title"),
        "H1 headings should not be included"
    );
    assert!(result.contains("## Phase 1"));
    assert!(result.contains("### Sub-section"));
}

// -------------------------------------------------------------------------
// fetch_plan_summary_for_session tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn test_fetch_plan_summary_session_not_found_returns_empty() {
    let state = AppState::new_sqlite_test();
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
    let state = AppState::new_sqlite_test();
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
    let state = AppState::new_sqlite_test();
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

    assert!(
        result.starts_with("Plan Structure:"),
        "Should start with Plan Structure:"
    );
    assert!(result.contains("## Phase 1: Setup"));
    assert!(result.contains("## Phase 2: Features"));
}

// ============================================================================
// Verification Gate Integration Tests — Scenarios 10-25
// ============================================================================

/// Shared setup: create artifact + session with plan, optionally set verification_status
/// and enable the proposal gate in the DB.
async fn setup_session_with_gate(
    state: &AppState,
    verification_status: &str,
    gate_enabled: bool,
) -> (IdeationSession, ArtifactId) {
    let project_id = ProjectId::new();

    let artifact = Artifact::new_inline("Plan", ArtifactType::Specification, "# Plan", "test");
    let artifact_id = artifact.id.clone();
    state.artifact_repo.create(artifact).await.unwrap();

    let session = IdeationSession::builder()
        .project_id(project_id)
        .title("Gate Test Session")
        .plan_artifact_id(artifact_id.clone())
        .build();
    state.ideation_session_repo.create(session.clone()).await.unwrap();

    // Apply verification_status and gate setting via raw SQL (both share the same SQLite conn)
    let sid = session.id.as_str().to_string();
    let status = verification_status.to_string();
    let gate: i64 = if gate_enabled { 1 } else { 0 };
    state
        .db
        .run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET verification_status = ?1 WHERE id = ?2",
                rusqlite::params![status, sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_proposals = ?1 WHERE id = 1",
                rusqlite::params![gate],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    (session, artifact_id)
}

/// Shared helper: create a proposal in a verified session (gate off) for use in update/delete tests.
async fn create_test_proposal(state: &AppState, session_id: &IdeationSessionId) -> TaskProposalId {
    let options = CreateProposalOptions {
        title: "Test Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };
    create_proposal_impl(state, session_id.clone(), options)
        .await
        .expect("test proposal creation should succeed")
        .id
}

// Scenario 10: HTTP create on unverified session + gate on → 400 (AppError::Validation with ProposalNotVerified message)
#[tokio::test]
async fn test_create_gate_blocks_unverified_when_enabled() {
    let state = AppState::new_sqlite_test();
    let (session, _artifact_id) = setup_session_with_gate(&state, "unverified", true).await;

    let options = CreateProposalOptions {
        title: "Blocked Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    let result = create_proposal_impl(&state, session.id.clone(), options).await;
    assert!(result.is_err(), "Create on Unverified+gate=on must fail");
    match result.unwrap_err() {
        AppError::Validation(msg) => {
            assert!(
                msg.contains("Cannot create proposals"),
                "Error must mention 'Cannot create proposals', got: {}",
                msg
            );
        }
        other => panic!("Expected AppError::Validation, got: {:?}", other),
    }
}

// Scenario 11: Tauri IPC parity — IPC and HTTP use the same create_proposal_impl,
// so the verification gate blocks with identical semantics (validates the refactor).
#[tokio::test]
async fn test_create_gate_ipc_parity_same_error_as_http() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "unverified", true).await;

    // Both IPC and HTTP call create_proposal_impl — the single enforcement point.
    let ipc_options = CreateProposalOptions {
        title: "IPC Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };
    let http_options = CreateProposalOptions {
        title: "HTTP Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    let ipc_result = create_proposal_impl(&state, session.id.clone(), ipc_options).await;
    let http_result = create_proposal_impl(&state, session.id.clone(), http_options).await;

    // Both must fail with the same error type (single enforcement point)
    assert!(ipc_result.is_err(), "IPC create on Unverified must fail");
    assert!(http_result.is_err(), "HTTP create on Unverified must fail");
    assert!(
        matches!(ipc_result.unwrap_err(), AppError::Validation(_)),
        "IPC must return Validation error"
    );
    assert!(
        matches!(http_result.unwrap_err(), AppError::Validation(_)),
        "HTTP must return Validation error"
    );
}

// Scenario 12: HTTP update during Reviewing + gate on → 400
#[tokio::test]
async fn test_update_gate_blocks_when_reviewing() {
    let state = AppState::new_sqlite_test();
    // First create with gate off to get a proposal
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;
    let proposal_id = create_test_proposal(&state, &session.id).await;

    // Now enable gate and set status to Reviewing
    let sid = session.id.as_str().to_string();
    state
        .db
        .run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET verification_status = 'reviewing' WHERE id = ?1",
                rusqlite::params![sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_proposals = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let options = UpdateProposalOptions {
        title: Some("Updated".to_string()),
        source: UpdateSource::Api,
        ..Default::default()
    };
    let result = update_proposal_impl(&state, &proposal_id, options).await;
    assert!(result.is_err(), "Update on Reviewing+gate=on must fail");
    match result.unwrap_err() {
        AppError::Validation(msg) => {
            assert!(
                msg.contains("Cannot update proposals"),
                "Error must mention 'Cannot update proposals', got: {}",
                msg
            );
        }
        other => panic!("Expected AppError::Validation, got: {:?}", other),
    }
}

// Scenario 13: HTTP delete during NeedsRevision + gate on → 400
#[tokio::test]
async fn test_delete_gate_blocks_when_needs_revision() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;
    let proposal_id = create_test_proposal(&state, &session.id).await;

    // Enable gate and set status to NeedsRevision
    let sid = session.id.as_str().to_string();
    state
        .db
        .run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET verification_status = 'needs_revision' WHERE id = ?1",
                rusqlite::params![sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_proposals = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let result = delete_proposal_impl(&state, proposal_id).await;
    assert!(result.is_err(), "Delete on NeedsRevision+gate=on must fail");
    match result.unwrap_err() {
        AppError::Validation(msg) => {
            assert!(
                msg.contains("Cannot delete proposals"),
                "Error must mention 'Cannot delete proposals', got: {}",
                msg
            );
        }
        other => panic!("Expected AppError::Validation, got: {:?}", other),
    }
}

// Scenario 14: Single event per operation — verify proposal appears exactly once in DB
// (no duplicate rows, confirming no double-emit causes duplicate writes).
// app_handle=None in tests, so events are silently skipped; impl is called once → one row.
#[tokio::test]
async fn test_create_proposal_inserted_exactly_once() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;

    let options = CreateProposalOptions {
        title: "Single Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    let proposal = create_proposal_impl(&state, session.id.clone(), options)
        .await
        .unwrap();

    // Verify exactly one proposal exists in DB
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session.id)
        .await
        .unwrap();
    assert_eq!(proposals.len(), 1, "Exactly one proposal should exist");
    assert_eq!(proposals[0].id, proposal.id);
}

// Scenario 15: IPC update sets user_modified=true on changed fields.
#[tokio::test]
async fn test_update_ipc_sets_user_modified() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;
    let proposal_id = create_test_proposal(&state, &session.id).await;

    let options = UpdateProposalOptions {
        title: Some("IPC Updated Title".to_string()),
        source: UpdateSource::TauriIpc,
        ..Default::default()
    };
    let updated = update_proposal_impl(&state, &proposal_id, options)
        .await
        .unwrap();

    assert!(
        updated.user_modified,
        "IPC update must set user_modified=true"
    );
}

// Scenario 16: API update does NOT set user_modified (agent-originated, no field tracking).
#[tokio::test]
async fn test_update_api_does_not_set_user_modified() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;
    let proposal_id = create_test_proposal(&state, &session.id).await;

    // Verify proposal starts with user_modified=false
    let initial = state
        .task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .unwrap()
        .unwrap();
    assert!(!initial.user_modified, "Proposal should start with user_modified=false");

    let options = UpdateProposalOptions {
        title: Some("API Updated Title".to_string()),
        source: UpdateSource::Api,
        ..Default::default()
    };
    let updated = update_proposal_impl(&state, &proposal_id, options)
        .await
        .unwrap();

    assert!(
        !updated.user_modified,
        "API update must NOT set user_modified"
    );
}

// Scenario 17: estimated_complexity roundtrip — create with complexity → stored correctly.
#[tokio::test]
async fn test_create_proposal_with_estimated_complexity_roundtrip() {
    let state = AppState::new_sqlite_test();
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;

    let options = CreateProposalOptions {
        title: "Complexity Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: Some("complex".to_string()),
    };
    let proposal = create_proposal_impl(&state, session.id.clone(), options)
        .await
        .unwrap();

    // Reload from DB and verify complexity is stored
    let reloaded = state
        .task_proposal_repo
        .get_by_id(&proposal.id)
        .await
        .unwrap()
        .expect("Proposal must be in DB");
    assert_eq!(
        reloaded.estimated_complexity.to_string().to_lowercase(),
        "complex",
        "Estimated complexity must round-trip correctly"
    );
}

// Scenario 18: assert_session_mutable on update — Archived session blocks update_proposal_impl.
#[tokio::test]
async fn test_update_proposal_on_archived_session_blocked() {
    let state = AppState::new_sqlite_test();
    let project_id = ProjectId::new();

    let artifact = Artifact::new_inline("Plan", ArtifactType::Specification, "# Plan", "test");
    let artifact_id = artifact.id.clone();
    state.artifact_repo.create(artifact).await.unwrap();

    // Create as Active, create proposal, then Archive the session
    let session = IdeationSession::builder()
        .project_id(project_id)
        .title("Archived Session")
        .plan_artifact_id(artifact_id)
        .build();
    let session_id = session.id.clone();
    state.ideation_session_repo.create(session).await.unwrap();
    let proposal_id = create_test_proposal(&state, &session_id).await;

    // Archive the session via SQL
    let sid = session_id.as_str().to_string();
    state
        .db
        .run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET status = 'archived' WHERE id = ?1",
                rusqlite::params![sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let options = UpdateProposalOptions {
        title: Some("Should Fail".to_string()),
        source: UpdateSource::Api,
        ..Default::default()
    };
    let result = update_proposal_impl(&state, &proposal_id, options).await;
    assert!(result.is_err(), "Update on Archived session must fail");
    assert!(
        matches!(result.unwrap_err(), AppError::Validation(_)),
        "Must return Validation error for Archived session"
    );
}

// Scenario 19: assert_session_mutable on delete — Accepted session blocks delete_proposal_impl.
// Also validates the Phase 1 bug fix: HTTP delete now guards via delete_proposal_impl.
#[tokio::test]
async fn test_delete_proposal_on_accepted_session_blocked() {
    let state = AppState::new_sqlite_test();
    let project_id = ProjectId::new();

    let artifact = Artifact::new_inline("Plan", ArtifactType::Specification, "# Plan", "test");
    let artifact_id = artifact.id.clone();
    state.artifact_repo.create(artifact).await.unwrap();

    let session = IdeationSession::builder()
        .project_id(project_id)
        .title("Accepted Session")
        .plan_artifact_id(artifact_id)
        .build();
    let session_id = session.id.clone();
    state.ideation_session_repo.create(session).await.unwrap();
    let proposal_id = create_test_proposal(&state, &session_id).await;

    // Accept the session via SQL
    let sid = session_id.as_str().to_string();
    state
        .db
        .run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET status = 'accepted' WHERE id = ?1",
                rusqlite::params![sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let result = delete_proposal_impl(&state, proposal_id).await;
    assert!(result.is_err(), "Delete on Accepted session must fail");
    assert!(
        matches!(result.unwrap_err(), AppError::Validation(_)),
        "Must return Validation error for Accepted session"
    );
}

// Scenario 20: Settings — require_verification_for_proposals roundtrip.
// Validates that the settings field persists and is read correctly.
#[tokio::test]
async fn test_settings_require_proposals_roundtrip_via_db() {
    let state = AppState::new_sqlite_test();

    // Enable require_verification_for_proposals
    state
        .db
        .run(|conn| {
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_proposals = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    // Read back via get_settings_sync inside a closure
    let proposals_enabled = state
        .db
        .run(|conn| {
            use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
            let s = get_settings_sync(conn)?;
            Ok(s.require_verification_for_proposals)
        })
        .await
        .unwrap();

    assert!(proposals_enabled, "require_verification_for_proposals must persist as true");

    // Disable and re-verify
    state
        .db
        .run(|conn| {
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_proposals = 0 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let proposals_disabled = state
        .db
        .run(|conn| {
            use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
            let s = get_settings_sync(conn)?;
            Ok(s.require_verification_for_proposals)
        })
        .await
        .unwrap();

    assert!(
        !proposals_disabled,
        "require_verification_for_proposals must persist as false"
    );
}

// Scenario 21: Both verification fields are independent — setting one doesn't affect the other.
#[tokio::test]
async fn test_settings_both_verification_fields_independent() {
    let state = AppState::new_sqlite_test();

    // Set accept=true, proposals=false
    state
        .db
        .run(|conn| {
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_accept = 1, require_verification_for_proposals = 0 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let (accept, proposals) = state
        .db
        .run(|conn| {
            use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
            let s = get_settings_sync(conn)?;
            Ok((s.require_verification_for_accept, s.require_verification_for_proposals))
        })
        .await
        .unwrap();

    assert!(accept, "require_verification_for_accept must be true");
    assert!(!proposals, "require_verification_for_proposals must be false");

    // Flip: accept=false, proposals=true
    state
        .db
        .run(|conn| {
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_accept = 0, require_verification_for_proposals = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let (accept2, proposals2) = state
        .db
        .run(|conn| {
            use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
            let s = get_settings_sync(conn)?;
            Ok((s.require_verification_for_accept, s.require_verification_for_proposals))
        })
        .await
        .unwrap();

    assert!(!accept2, "require_verification_for_accept must be false");
    assert!(proposals2, "require_verification_for_proposals must be true");
}

// Scenario 22: require_verification_for_accept roundtrip — validates the hardcoded-false bug fix.
// The settings repo used to return hardcoded `false` for this field regardless of DB value.
#[tokio::test]
async fn test_settings_require_accept_roundtrip_true_case() {
    let state = AppState::new_sqlite_test();

    state
        .db
        .run(|conn| {
            conn.execute(
                "UPDATE ideation_settings SET require_verification_for_accept = 1 WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    let accept_enabled = state
        .db
        .run(|conn| {
            use crate::infrastructure::sqlite::sqlite_ideation_settings_repo::get_settings_sync;
            let s = get_settings_sync(conn)?;
            Ok(s.require_verification_for_accept)
        })
        .await
        .unwrap();

    assert!(
        accept_enabled,
        "require_verification_for_accept must round-trip as true (validates bug fix)"
    );
}

// Scenario 23: Gate allows all ops when gate is off — create always succeeds regardless of status.
#[tokio::test]
async fn test_create_gate_off_allows_any_status() {
    let state = AppState::new_sqlite_test();
    // Gate off (default), session in Reviewing state
    let (session, _) = setup_session_with_gate(&state, "reviewing", false).await;

    let options = CreateProposalOptions {
        title: "Gate Off Proposal".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    let result = create_proposal_impl(&state, session.id.clone(), options).await;
    assert!(
        result.is_ok(),
        "Create must succeed when gate=off, got: {:?}",
        result.err()
    );
}

// Scenario 24: Concurrent creates produce unique sort_orders (TOCTOU prevention via transaction lock).
#[tokio::test]
async fn test_concurrent_creates_produce_unique_sort_orders() {
    use std::collections::HashSet;
    use std::sync::Arc;

    let state = Arc::new(AppState::new_sqlite_test());
    let (session, _) = setup_session_with_gate(&state, "verified", false).await;
    let session_id = session.id.clone();

    let make_options = |n: u32| CreateProposalOptions {
        title: format!("Concurrent Proposal {}", n),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    // Run 3 creates concurrently via tokio::join! (they share the same mutex-serialized DB conn)
    let (r1, r2, r3) = tokio::join!(
        create_proposal_impl(&state, session_id.clone(), make_options(1)),
        create_proposal_impl(&state, session_id.clone(), make_options(2)),
        create_proposal_impl(&state, session_id.clone(), make_options(3)),
    );

    let p1 = r1.expect("concurrent create 1 must succeed");
    let p2 = r2.expect("concurrent create 2 must succeed");
    let p3 = r3.expect("concurrent create 3 must succeed");

    // All sort_orders must be unique (no TOCTOU duplicates)
    let orders: HashSet<i32> = [p1.sort_order, p2.sort_order, p3.sort_order]
        .iter()
        .cloned()
        .collect();
    assert_eq!(orders.len(), 3, "All 3 sort_orders must be unique, got: {:?}", orders);

    // Verify all 3 proposals exist in DB
    let all_proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .unwrap();
    assert_eq!(all_proposals.len(), 3, "All 3 proposals must be in DB");
}

// Scenario 25: Concurrent create during status transition — TOCTOU prevention.
// If status changes to Reviewing while create is queued, the transaction reads the
// current status atomically — either succeeds (Verified before change) or fails (Reviewing after).
// No partial state is possible.
#[tokio::test]
async fn test_concurrent_create_during_status_transition_no_partial_state() {
    use std::sync::Arc;

    let state = Arc::new(AppState::new_sqlite_test());
    let (session, _) = setup_session_with_gate(&state, "verified", true).await;
    let session_id = session.id.clone();

    let options = CreateProposalOptions {
        title: "Race Condition Test".to_string(),
        description: None,
        category: ProposalCategory::Feature,
        suggested_priority: Priority::Medium,
        steps: None,
        acceptance_criteria: None,
        estimated_complexity: None,
    };

    // Concurrently: attempt create AND change status to Reviewing
    let sid = session_id.as_str().to_string();
    let (create_result, transition_result) = tokio::join!(
        create_proposal_impl(&state, session_id.clone(), options),
        state.db.run(move |conn| {
            conn.execute(
                "UPDATE ideation_sessions SET verification_status = 'reviewing' WHERE id = ?1",
                rusqlite::params![sid],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(())
        }),
    );

    // Transition must always succeed
    transition_result.expect("Status transition must succeed");

    // Create either succeeded (ran before transition) or failed (ran after transition)
    // — never a partial/corrupt state
    match create_result {
        Ok(proposal) => {
            // Proposal was created; verify it's in DB (no partial write)
            let in_db = state
                .task_proposal_repo
                .get_by_id(&proposal.id)
                .await
                .unwrap();
            assert!(in_db.is_some(), "Created proposal must be in DB");
        }
        Err(AppError::Validation(_)) => {
            // Gate blocked the create (status was already Reviewing when transaction ran)
            // No partial state — verify proposal count is 0
            let proposals = state
                .task_proposal_repo
                .get_by_session(&session_id)
                .await
                .unwrap();
            assert_eq!(proposals.len(), 0, "No partial proposals should exist");
        }
        Err(other) => panic!("Unexpected error type: {:?}", other),
    }
}
