use super::*;

#[test]
fn built_in_registry_tracks_provider_model_effort_compatibility() {
    let snapshot = AgentModelRegistrySnapshot::merged(Vec::new());

    let opus = snapshot
        .find_enabled(AgentHarnessKind::Claude, "opus")
        .expect("opus should be registered");
    assert_eq!(
        opus.supported_efforts,
        vec![
            LogicalEffort::Low,
            LogicalEffort::Medium,
            LogicalEffort::High,
            LogicalEffort::XHigh,
            LogicalEffort::Max,
        ]
    );
    assert_eq!(opus.default_effort, LogicalEffort::XHigh);

    let codex_mini = snapshot
        .find_enabled(AgentHarnessKind::Codex, "gpt-5.4-mini")
        .expect("gpt-5.4-mini should be registered");
    assert_eq!(
        codex_mini.supported_efforts,
        vec![
            LogicalEffort::Low,
            LogicalEffort::Medium,
            LogicalEffort::High
        ]
    );
    assert_eq!(codex_mini.default_effort, LogicalEffort::Medium);
}

#[test]
fn custom_models_override_built_ins() {
    let snapshot = AgentModelRegistrySnapshot::merged(vec![AgentModelDefinition::custom(
        AgentHarnessKind::Codex,
        "gpt-5.5",
        "Internal GPT-5.5",
        "Internal GPT-5.5",
        None,
        vec![LogicalEffort::Low, LogicalEffort::Medium],
        LogicalEffort::Medium,
        true,
    )]);

    let model = snapshot
        .find_enabled(AgentHarnessKind::Codex, "gpt-5.5")
        .expect("custom override should remain enabled");
    assert_eq!(model.source, AgentModelSource::Custom);
    assert_eq!(model.menu_label, "Internal GPT-5.5");
    assert_eq!(
        model.supported_efforts,
        vec![LogicalEffort::Low, LogicalEffort::Medium]
    );
}
