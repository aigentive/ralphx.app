use super::*;
use crate::application::AppState;
use crate::domain::entities::{
    Artifact, ArtifactType, IdeationSession, IdeationSessionStatus, ProjectId, ProposalCategory,
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
        category: ProposalCategory::Feature,
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
        category: ProposalCategory::Feature,
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
        category: ProposalCategory::Feature,
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
        .fold(String::new(), |mut acc, i| {
            use std::fmt::Write;
            writeln!(acc, "## Phase {}: Some very long phase title with lots of words here", i).unwrap();
            acc
        });
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
