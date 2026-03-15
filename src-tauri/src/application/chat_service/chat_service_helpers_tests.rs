use super::*;
use crate::infrastructure::agents::claude::agent_names::AGENT_PLAN_VERIFIER;

#[test]
fn test_resolve_agent_review_approved_returns_review_history() {
    let agent = resolve_agent(&ChatContextType::Review, Some("approved"));
    assert_eq!(agent, AGENT_REVIEW_HISTORY);
}

#[test]
fn test_resolve_agent_review_passed_returns_review_chat() {
    let agent = resolve_agent(&ChatContextType::Review, Some("review_passed"));
    assert_eq!(agent, AGENT_REVIEW_CHAT);
}

#[test]
fn test_resolve_agent_review_default_returns_reviewer() {
    let agent = resolve_agent(&ChatContextType::Review, None);
    assert_eq!(agent, AGENT_REVIEWER);
}

#[test]
fn test_resolve_agent_review_other_status_returns_reviewer() {
    let agent = resolve_agent(&ChatContextType::Review, Some("reviewing"));
    assert_eq!(agent, AGENT_REVIEWER);
}

#[test]
fn test_resolve_agent_ideation_accepted_returns_readonly() {
    let agent = resolve_agent(&ChatContextType::Ideation, Some("accepted"));
    assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION_READONLY);
}

#[test]
fn test_resolve_agent_ideation_verification_returns_plan_verifier() {
    // Verification sessions route to plan-verifier (prevents infinite cascade)
    let agent = resolve_agent(&ChatContextType::Ideation, Some("verification"));
    assert_eq!(agent, AGENT_PLAN_VERIFIER);
}

#[test]
fn test_resolve_agent_ideation_verification_overrides_team_mode() {
    // Verification purpose takes priority over team_mode
    let agent =
        resolve_agent_with_team_mode(&ChatContextType::Ideation, Some("verification"), true);
    assert_eq!(agent, AGENT_PLAN_VERIFIER);
}

#[test]
fn test_resolve_agent_team_mode_ideation_returns_team_lead() {
    let agent = resolve_agent_with_team_mode(&ChatContextType::Ideation, None, true);
    assert_eq!(agent, AGENT_IDEATION_TEAM_LEAD);
}

#[test]
fn test_resolve_agent_team_mode_execution_returns_worker_team() {
    let agent = resolve_agent_with_team_mode(&ChatContextType::TaskExecution, None, true);
    assert_eq!(agent, AGENT_WORKER_TEAM);
}

#[test]
fn test_resolve_agent_team_mode_project_falls_back_to_default() {
    // Contexts without team variants fall back to defaults
    let agent = resolve_agent_with_team_mode(&ChatContextType::Project, None, true);
    assert_eq!(agent, AGENT_CHAT_PROJECT);
}

#[test]
fn test_resolve_agent_team_mode_false_returns_default() {
    let agent = resolve_agent_with_team_mode(&ChatContextType::Ideation, None, false);
    assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION);
}

#[test]
fn test_resolve_agent_team_mode_status_overrides_team() {
    // Status-specific rules take priority over team_mode
    let agent = resolve_agent_with_team_mode(&ChatContextType::Ideation, Some("accepted"), true);
    assert_eq!(agent, AGENT_ORCHESTRATOR_IDEATION_READONLY);
}

#[test]
fn test_context_type_to_process_mapping() {
    assert_eq!(
        context_type_to_process(&ChatContextType::Ideation),
        "ideation"
    );
    assert_eq!(context_type_to_process(&ChatContextType::Task), "task");
    assert_eq!(
        context_type_to_process(&ChatContextType::Project),
        "project"
    );
    assert_eq!(
        context_type_to_process(&ChatContextType::TaskExecution),
        "execution"
    );
    assert_eq!(context_type_to_process(&ChatContextType::Review), "review");
    assert_eq!(context_type_to_process(&ChatContextType::Merge), "merge");
}
