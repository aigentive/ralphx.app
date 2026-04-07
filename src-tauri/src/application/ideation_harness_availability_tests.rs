use super::ideation_harness_availability::{
    build_ideation_lane_harness_availability, HarnessRuntimeProbe, ResolvedLaneHarnessConfig,
};
use crate::domain::agents::{AgentHarnessKind, AgentLane};

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

    let availability =
        build_ideation_lane_harness_availability(config, &claude_probe, &codex_probe);

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

    let availability =
        build_ideation_lane_harness_availability(config, &claude_probe, &codex_probe);

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
        &unavailable_probe("Claude CLI not found"),
        &unavailable_probe("Codex CLI not found"),
    );

    assert_eq!(availability.effective_harness, AgentHarnessKind::Claude);
    assert!(!availability.fallback_activated);
    assert!(!availability.available);
    assert_eq!(availability.error.as_deref(), Some("Claude CLI not found"));
}
