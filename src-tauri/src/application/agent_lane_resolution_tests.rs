use std::sync::Arc;

use super::agent_lane_resolution::resolve_agent_spawn_settings;
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

fn claude_lane_settings(
    model: &str,
    effort: Option<LogicalEffort>,
    approval_policy: Option<&str>,
    sandbox_mode: Option<&str>,
) -> AgentLaneSettings {
    AgentLaneSettings {
        harness: AgentHarnessKind::Claude,
        model: Some(model.to_string()),
        effort,
        approval_policy: approval_policy.map(str::to_string),
        sandbox_mode: sandbox_mode.map(str::to_string),
        fallback_harness: None,
    }
}

fn codex_lane_settings(
    model: &str,
    effort: Option<LogicalEffort>,
    approval_policy: Option<&str>,
    sandbox_mode: Option<&str>,
) -> AgentLaneSettings {
    AgentLaneSettings {
        harness: AgentHarnessKind::Codex,
        model: Some(model.to_string()),
        effort,
        approval_policy: approval_policy.map(str::to_string),
        sandbox_mode: sandbox_mode.map(str::to_string),
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
            &claude_lane_settings(
                "opus",
                Some(LogicalEffort::XHigh),
                Some("on_request"),
                Some("workspace_write"),
            ),
        )
        .await
        .expect("lane upsert should succeed");
    lane_repo
        .upsert_for_project(
            "proj-1",
            AgentLane::IdeationSubagent,
            &claude_lane_settings("haiku", None, None, None),
        )
        .await
        .expect("subagent lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "orchestrator-ideation",
        Some("proj-1"),
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Claude));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Claude);
    assert_eq!(resolved.configured_model.as_deref(), Some("opus"));
    assert_eq!(resolved.configured_logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.configured_approval_policy.as_deref(), Some("on_request"));
    assert_eq!(resolved.configured_sandbox_mode.as_deref(), Some("workspace_write"));
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.claude_effort.as_deref(), Some("max"));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on_request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace_write"));
    assert_eq!(resolved.configured_subagent_model_cap.as_deref(), Some("haiku"));
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

    let resolved = resolve_agent_spawn_settings(
        "orchestrator-ideation",
        Some("proj-2"),
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, None);
    assert_eq!(resolved.configured_model, None);
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::High));
    assert_eq!(resolved.claude_effort.as_deref(), Some("high"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("haiku"));
}

#[tokio::test]
async fn codex_lane_selection_uses_codex_lane_settings() {
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
            &codex_lane_settings(
                "gpt-5.4",
                Some(LogicalEffort::XHigh),
                Some("on_request"),
                Some("workspace_write"),
            ),
        )
        .await
        .expect("codex lane upsert should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationSubagent,
            &codex_lane_settings(
                "gpt-5.4-mini",
                Some(LogicalEffort::Medium),
                Some("never"),
                Some("danger_full_access"),
            ),
        )
        .await
        .expect("codex subagent lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "orchestrator-ideation",
        None,
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Codex));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.configured_model.as_deref(), Some("gpt-5.4"));
    assert_eq!(resolved.configured_logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.configured_approval_policy.as_deref(), Some("on_request"));
    assert_eq!(resolved.configured_sandbox_mode.as_deref(), Some("workspace_write"));
    assert_eq!(resolved.model, "gpt-5.4");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.claude_effort.as_deref(), Some("max"));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on_request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace_write"));
    assert_eq!(resolved.configured_subagent_model_cap.as_deref(), Some("gpt-5.4-mini"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("gpt-5.4-mini"));
}

#[tokio::test]
async fn codex_primary_lane_without_model_or_effort_uses_phase1_defaults() {
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
            &AgentLaneSettings {
                harness: AgentHarnessKind::Codex,
                model: None,
                effort: None,
                approval_policy: None,
                sandbox_mode: None,
                fallback_harness: Some(AgentHarnessKind::Claude),
            },
        )
        .await
        .expect("codex lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "orchestrator-ideation",
        None,
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.model, "gpt-5.4");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on-request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace-write"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("gpt-5.4-mini"));
}

#[tokio::test]
async fn codex_verifier_lane_without_model_or_effort_uses_phase1_defaults() {
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
            &AgentLaneSettings {
                harness: AgentHarnessKind::Codex,
                model: None,
                effort: None,
                approval_policy: None,
                sandbox_mode: None,
                fallback_harness: Some(AgentHarnessKind::Claude),
            },
        )
        .await
        .expect("verifier codex lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "plan-verifier",
        None,
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.model, "gpt-5.4-mini");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::Medium));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on-request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace-write"));
    assert_eq!(resolved.subagent_model_cap.as_deref(), Some("gpt-5.4-mini"));
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
            &claude_lane_settings("opus", Some(LogicalEffort::High), None, None),
        )
        .await
        .expect("verifier lane upsert should succeed");
    lane_repo
        .upsert_global(
            AgentLane::IdeationVerifierSubagent,
            &claude_lane_settings("haiku", None, None, None),
        )
        .await
        .expect("verifier subagent lane upsert should succeed");

    let verifier = resolve_agent_spawn_settings(
        "plan-verifier",
        None,
        ChatContextType::Ideation,
        None,
        None,
        Some(&lane_repo),
        Some(&model_repo),
        Some(&effort_repo),
    )
    .await;

    assert_eq!(verifier.model, "opus");
    assert_eq!(verifier.logical_effort, Some(LogicalEffort::High));
    assert_eq!(verifier.claude_effort.as_deref(), Some("high"));
    assert_eq!(verifier.subagent_model_cap.as_deref(), Some("haiku"));
}

#[tokio::test]
async fn execution_worker_lane_can_resolve_codex_settings() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());

    lane_repo
        .upsert_global(
            AgentLane::ExecutionWorker,
            &codex_lane_settings(
                "gpt-5.4",
                Some(LogicalEffort::High),
                Some("on-request"),
                Some("workspace-write"),
            ),
        )
        .await
        .expect("execution worker lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "worker",
        None,
        ChatContextType::TaskExecution,
        None,
        None,
        Some(&lane_repo),
        None,
        None,
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Codex));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.model, "gpt-5.4");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::High));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on-request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace-write"));
    assert_eq!(resolved.subagent_model_cap, None);
}

#[tokio::test]
async fn execution_worker_codex_without_model_uses_generic_codex_defaults() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());

    lane_repo
        .upsert_global(
            AgentLane::ExecutionWorker,
            &AgentLaneSettings {
                harness: AgentHarnessKind::Codex,
                model: None,
                effort: None,
                approval_policy: None,
                sandbox_mode: None,
                fallback_harness: Some(AgentHarnessKind::Claude),
            },
        )
        .await
        .expect("execution worker codex lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "worker",
        None,
        ChatContextType::TaskExecution,
        None,
        None,
        Some(&lane_repo),
        None,
        None,
    )
    .await;

    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.model, "gpt-5.4");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::XHigh));
    assert_eq!(resolved.approval_policy.as_deref(), Some("on-request"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("workspace-write"));
}

#[tokio::test]
async fn reexecuting_task_execution_uses_reexecutor_lane_settings() {
    let lane_repo: Arc<dyn AgentLaneSettingsRepository> =
        Arc::new(MemoryAgentLaneSettingsRepository::new());

    lane_repo
        .upsert_global(
            AgentLane::ExecutionReexecutor,
            &codex_lane_settings(
                "gpt-5.4-mini",
                Some(LogicalEffort::Medium),
                Some("never"),
                Some("read-only"),
            ),
        )
        .await
        .expect("execution reexecutor lane upsert should succeed");

    let resolved = resolve_agent_spawn_settings(
        "worker",
        None,
        ChatContextType::TaskExecution,
        Some("re_executing"),
        None,
        Some(&lane_repo),
        None,
        None,
    )
    .await;

    assert_eq!(resolved.configured_harness, Some(AgentHarnessKind::Codex));
    assert_eq!(resolved.effective_harness, AgentHarnessKind::Codex);
    assert_eq!(resolved.model, "gpt-5.4-mini");
    assert_eq!(resolved.logical_effort, Some(LogicalEffort::Medium));
    assert_eq!(resolved.approval_policy.as_deref(), Some("never"));
    assert_eq!(resolved.sandbox_mode.as_deref(), Some("read-only"));
}
