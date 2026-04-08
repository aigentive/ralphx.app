use super::ideation_harness_availability::{
    build_ideation_lane_harness_availability, standard_harness_probe_registry,
    validate_claude_runtime_path, HarnessRuntimeProbe, IdeationLaneHarnessAvailability,
    ResolvedLaneHarnessConfig,
};
use crate::domain::agents::{AgentHarnessKind, AgentLane};
use std::collections::HashMap;

fn unavailable_probe(error: &str) -> HarnessRuntimeProbe {
    HarnessRuntimeProbe {
        binary_path: None,
        binary_found: false,
        probe_succeeded: false,
        available: false,
        missing_core_exec_features: Vec::new(),
        error: Some(error.to_string()),
    }
}

fn probe_map(
    claude_probe: HarnessRuntimeProbe,
    codex_probe: HarnessRuntimeProbe,
) -> HashMap<AgentHarnessKind, HarnessRuntimeProbe> {
    HashMap::from([
        (AgentHarnessKind::Claude, claude_probe),
        (AgentHarnessKind::Codex, codex_probe),
    ])
}

#[test]
fn codex_lane_uses_codex_when_core_exec_support_is_available() {
    let config = ResolvedLaneHarnessConfig {
        lane: AgentLane::IdeationPrimary,
        configured_harness: Some(AgentHarnessKind::Codex),
        fallback_harness: Some(AgentHarnessKind::Claude),
    };
    let claude_probe = HarnessRuntimeProbe {
        binary_path: Some("/opt/homebrew/bin/claude".to_string()),
        binary_found: true,
        probe_succeeded: true,
        available: true,
        missing_core_exec_features: Vec::new(),
        error: None,
    };
    let codex_probe = HarnessRuntimeProbe {
        binary_path: Some("/opt/homebrew/bin/codex".to_string()),
        binary_found: true,
        probe_succeeded: true,
        available: true,
        missing_core_exec_features: Vec::new(),
        error: None,
    };

    let availability = build_ideation_lane_harness_availability(
        config,
        &probe_map(claude_probe, codex_probe),
    );

    assert_eq!(availability.effective_harness, AgentHarnessKind::Codex);
    assert!(!availability.fallback_activated);
    assert!(availability.available);
    assert_eq!(
        availability.binary_path.as_deref(),
        Some("/opt/homebrew/bin/codex")
    );
}

#[test]
fn codex_lane_falls_back_to_claude_when_requested_and_codex_is_unavailable() {
    let config = ResolvedLaneHarnessConfig {
        lane: AgentLane::IdeationVerifier,
        configured_harness: Some(AgentHarnessKind::Codex),
        fallback_harness: Some(AgentHarnessKind::Claude),
    };
    let claude_probe = HarnessRuntimeProbe {
        binary_path: Some("/opt/homebrew/bin/claude".to_string()),
        binary_found: true,
        probe_succeeded: true,
        available: true,
        missing_core_exec_features: Vec::new(),
        error: None,
    };
    let codex_probe = HarnessRuntimeProbe {
        binary_path: Some("/opt/homebrew/bin/codex".to_string()),
        binary_found: true,
        probe_succeeded: true,
        available: false,
        missing_core_exec_features: vec!["json_output".to_string()],
        error: Some("Codex CLI is missing required capability: json_output".to_string()),
    };

    let availability = build_ideation_lane_harness_availability(
        config,
        &probe_map(claude_probe, codex_probe),
    );

    assert_eq!(availability.effective_harness, AgentHarnessKind::Claude);
    assert!(availability.fallback_activated);
    assert!(availability.available);
    assert_eq!(
        availability.error.as_deref(),
        Some("Codex CLI is missing required capability: json_output")
    );
    assert_eq!(
        availability.missing_core_exec_features,
        vec!["json_output".to_string()]
    );
}

#[test]
fn default_lane_without_configuration_defaults_to_claude() {
    let config = ResolvedLaneHarnessConfig {
        lane: AgentLane::IdeationSubagent,
        configured_harness: None,
        fallback_harness: None,
    };

    let availability = build_ideation_lane_harness_availability(
        config,
        &probe_map(
            unavailable_probe("Claude CLI not found"),
            unavailable_probe("Codex CLI not found"),
        ),
    );

    assert_eq!(availability.effective_harness, AgentHarnessKind::Claude);
    assert!(!availability.fallback_activated);
    assert!(!availability.available);
    assert_eq!(availability.error.as_deref(), Some("Claude CLI not found"));
}

#[test]
fn validate_claude_runtime_path_accepts_available_claude() {
    let availability = IdeationLaneHarnessAvailability {
        lane: AgentLane::IdeationPrimary,
        configured_harness: Some(AgentHarnessKind::Claude),
        fallback_harness: None,
        effective_harness: AgentHarnessKind::Claude,
        fallback_activated: false,
        binary_path: None,
        binary_found: true,
        probe_succeeded: true,
        available: true,
        missing_core_exec_features: Vec::new(),
        error: None,
    };

    assert!(validate_claude_runtime_path(&availability, "unified ideation").is_ok());
}

#[test]
fn validate_claude_runtime_path_rejects_available_codex() {
    let availability = IdeationLaneHarnessAvailability {
        lane: AgentLane::IdeationPrimary,
        configured_harness: Some(AgentHarnessKind::Codex),
        fallback_harness: None,
        effective_harness: AgentHarnessKind::Codex,
        fallback_activated: false,
        binary_path: None,
        binary_found: true,
        probe_succeeded: true,
        available: true,
        missing_core_exec_features: Vec::new(),
        error: None,
    };

    let error = validate_claude_runtime_path(&availability, "unified ideation").unwrap_err();
    assert!(error.contains("unified ideation"));
    assert!(error.contains("Claude runtime"));
}

#[test]
fn standard_harness_probe_registry_keys_explicit_harnesses() {
    let registry = standard_harness_probe_registry();

    assert!(registry.contains_key(&AgentHarnessKind::Claude));
    assert!(registry.contains_key(&AgentHarnessKind::Codex));
}

#[test]
fn missing_requested_probe_does_not_silently_fall_back_to_default_probe() {
    let config = ResolvedLaneHarnessConfig {
        lane: AgentLane::IdeationPrimary,
        configured_harness: Some(AgentHarnessKind::Codex),
        fallback_harness: None,
    };
    let probes = HashMap::from([(
        AgentHarnessKind::Claude,
        HarnessRuntimeProbe {
            binary_path: Some("/opt/homebrew/bin/claude".to_string()),
            binary_found: true,
            probe_succeeded: true,
            available: true,
            missing_core_exec_features: Vec::new(),
            error: None,
        },
    )]);

    let availability = build_ideation_lane_harness_availability(config, &probes);

    assert_eq!(availability.effective_harness, AgentHarnessKind::Codex);
    assert!(!availability.available);
    assert_eq!(
        availability.error.as_deref(),
        Some("No harness probe registered for codex")
    );
}
