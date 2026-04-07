use crate::application::ideation_harness_availability::IdeationLaneHarnessAvailability;
use crate::commands::ideation_commands::ideation_commands_orchestrator::validate_deprecated_orchestrator_path;
use crate::domain::agents::{AgentHarnessKind, AgentLane};

fn availability(
    effective_harness: AgentHarnessKind,
    available: bool,
    error: Option<&str>,
) -> IdeationLaneHarnessAvailability {
    IdeationLaneHarnessAvailability {
        lane: AgentLane::IdeationPrimary,
        configured_harness: Some(effective_harness),
        fallback_harness: None,
        effective_harness,
        fallback_activated: false,
        binary_path: None,
        binary_found: available,
        probe_succeeded: available,
        available,
        missing_core_exec_features: Vec::new(),
        error: error.map(str::to_string),
    }
}

#[test]
fn deprecated_orchestrator_path_accepts_claude() {
    let result = validate_deprecated_orchestrator_path(&availability(
        AgentHarnessKind::Claude,
        true,
        None,
    ));

    assert!(result.is_ok());
}

#[test]
fn deprecated_orchestrator_path_rejects_unavailable_harnesses() {
    let result = validate_deprecated_orchestrator_path(&availability(
        AgentHarnessKind::Claude,
        false,
        Some("Claude CLI not found"),
    ));

    assert_eq!(result.unwrap_err(), "Claude CLI not found");
}

#[test]
fn deprecated_orchestrator_path_rejects_effective_codex() {
    let result = validate_deprecated_orchestrator_path(&availability(
        AgentHarnessKind::Codex,
        true,
        None,
    ));

    assert!(
        result
            .unwrap_err()
            .contains("deprecated orchestrator path still routes through the Claude runtime")
    );
}
