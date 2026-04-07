use std::sync::Arc;

use super::agent_lane_resolution::resolve_claude_spawn_settings;
use crate::domain::agents::{AgentHarnessKind, AgentLane, AgentLaneSettings, LogicalEffort};
use crate::domain::entities::ChatContextType;
use crate::domain::repositories::{
    AgentLaneSettingsRepository, IdeationEffortSettingsRepository,
    IdeationModelSettingsRepository,
};
use crate::infrastructure::memory::{
    MemoryAgentLaneSettingsRepository, MemoryIdeationEffortSettingsRepository,
    MemoryIdeationModelSettingsRepository,
};

fn claude_lane_settings(model: &str, effort: Option<LogicalEffort>) -> AgentLaneSettings {
    AgentLaneSettings {
        harness: AgentHarnessKind::Claude,
        model: Some(model.to_string()),
        effort,
        approval_policy: None,
        sandbox_mode: None,
        fallback_harness: None,
    }
}

fn codex_lane_settings(model: &str, effort: Option<LogicalEffort>) -> AgentLaneSettings {
    AgentLaneSettings {
        harness: AgentHarnessKind::Codex,
        model: Some(model.to_string()),
        effort,
        approval_policy: None,
        sandbox_mode: None,
        fallback_harness: Some(AgentHarnessKind::Claude),
    }
}

#[tokio::test]
async fn lane_row_with_claude_harness_overrides_legacy_model_and_effort() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());
    let model_repo: Arc<dyn IdeationModelSettingsRepository> =
        Arc::new(MemoryIdeationModelSettingsRepository::new());
    let effort_repo: Arc<dyn IdeationEffortSettingsRepository> =
        Arc::new(MemoryIdeationEffortSettingsRepository::new());

    model_repo
        .upsert_global("haiku", "haiku", "haiku", "haiku")
        .await
        .expect("legacy model seed should succeed");
    effort_repo
        .upsert(None, "low", "low")
        .await
        .expect("legacy effort seed should succeed");
    lane_repo
        .upsert_for_project(
            "proj-1",
            AgentLane::IdeationPrimary,
            &claude_lane_settings("opus", Some(LogicalEffort::XHigh)),
        )
        .await
        .expect("lane upsert should succeed");
    lane_repo
        .upsert_for_project(
            "proj-1",
            AgentLane::IdeationSubagent,
            &claude_lane_settings("haiku", None),
        )
        .await
        .expect("subagent lane upsert should succeed");

    let resolved = resolve_claude_spawn_settings(
        "orchestrator-ideation",
        Some("proj-1"),
        ChatContextType::Ideation,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Claude));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Claude);
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.effort.as_deref(), Some("max"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("haiku"));
}

#[tokio::test]
async fn missing_lane_row_falls_back_to_legacy_ideation_settings() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());
    let model_repo: Arc<dyn IdeationModelSettingsRepository> =
        Arc::new(MemoryIdeationModelSettingsRepository::new());
    let effort_repo: Arc<dyn IdeationEffortSettingsRepository> =
        Arc::new(MemoryIdeationEffortSettingsRepository::new());

    model_repo
        .upsert_for_project("proj-2", "opus", "haiku", "sonnet", "haiku")
        .await
        .expect("legacy project model seed should succeed");
    effort_repo
        .upsert(Some("proj-2"), "high", "medium")
        .await
        .expect("legacy project effort seed should succeed");

    let resolved = resolve_claude_spawn_settings(
        "orchestrator-ideation",
        Some("proj-2"),
        ChatContextType::Ideation,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, None);
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.effort.as_deref(), Some("high"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("haiku"));
}

#[tokio::test]
async fn codex_lane_selection_degrades_to_legacy_claude_settings() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());
    let model_repo: Arc<dyn IdeationModelSettingsRepository> =
        Arc::new(MemoryIdeationModelSettingsRepository::new());
    let effort_repo: Arc<dyn IdeationEffortSettingsRepository> =
        Arc::new(MemoryIdeationEffortSettingsRepository::new());

    model_repo
        .upsert_global("sonnet", "opus", "haiku", "haiku")
        .await
        .expect("legacy model seed should succeed");
    effort_repo
        .upsert(None, "medium", "high")
        .await
        .expect("legacy effort seed should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationPrimary,
            &codex_lane_settings("gpt-5.4", Some(LogicalEffort::XHigh)),
        )
        .await
        .expect("codex lane upsert should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationSubagent,
            &codex_lane_settings("gpt-5.4-mini", Some(LogicalEffort::Medium)),
        )
        .await
        .expect("codex subagent lane upsert should succeed");

    let resolved = resolve_claude_spawn_settings(
        "orchestrator-ideation",
        None,
        ChatContextType::Ideation,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Codex));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Claude);
    assert_eq!(resolved.model, "sonnet");
    assert_eq!(resolved.effort.as_deref(), Some("medium"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("haiku"));
}

#[tokio::test]
async fn verifier_and_primary_subagent_caps_use_lane_rows_when_claude_is_selected() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());
    let model_repo: Arc<dyn IdeationModelSettingsRepository> =
        Arc::new(MemoryIdeationModelSettingsRepository::new());
    let effort_repo: Arc<dyn IdeationEffortSettingsRepository> =
        Arc::new(MemoryIdeationEffortSettingsRepository::new());

    model_repo
        .upsert_global("sonnet", "opus", "haiku", "haiku")
        .await
        .expect("legacy model seed should succeed");
    effort_repo
        .upsert(None, "medium", "high")
        .await
        .expect("legacy effort seed should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationVerifier,
            &claude_lane_settings("opus", Some(LogicalEffort::High)),
        )
        .await
        .expect("verifier lane upsert should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationVerifierSubagent,
            &claude_lane_settings("haiku", None),
        )
        .await
        .expect("verifier subagent lane upsert should succeed");

    let verifier = resolve_claude_spawn_settings(
        "plan-verifier",
        None,
        ChatContextType::Ideation,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(verifier.model, "opus");
    assert_eq!(verifier.effort.as_deref(), Some("high"));
    assert_eq!(verifier.subagent_model_cap.as_deref(), Some("haiku"));
}
