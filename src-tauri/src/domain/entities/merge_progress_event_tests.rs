use super::*;

use super::*;

#[test]
fn merge_phase_serializes_to_snake_case() {
    assert_eq!(
        serde_json::to_string(&MergePhase::WorktreeSetup).unwrap(),
        "\"worktree_setup\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::ProgrammaticMerge).unwrap(),
        "\"programmatic_merge\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::Typecheck).unwrap(),
        "\"typecheck\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::Lint).unwrap(),
        "\"lint\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::Clippy).unwrap(),
        "\"clippy\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::Test).unwrap(),
        "\"test\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::Finalize).unwrap(),
        "\"finalize\""
    );
}

#[test]
fn merge_phase_deserializes_from_snake_case() {
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"worktree_setup\"").unwrap(),
        MergePhase::WorktreeSetup
    );
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"programmatic_merge\"").unwrap(),
        MergePhase::ProgrammaticMerge
    );
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"typecheck\"").unwrap(),
        MergePhase::Typecheck
    );
}

#[test]
fn merge_phase_status_serializes() {
    assert_eq!(
        serde_json::to_string(&MergePhaseStatus::Started).unwrap(),
        "\"started\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhaseStatus::Passed).unwrap(),
        "\"passed\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhaseStatus::Failed).unwrap(),
        "\"failed\""
    );
}

#[test]
fn merge_progress_event_new_sets_fields() {
    let event = MergeProgressEvent::new(
        "task-123".to_string(),
        MergePhase::Typecheck,
        MergePhaseStatus::Started,
        "Running type check".to_string(),
    );

    assert_eq!(event.task_id, "task-123");
    assert_eq!(event.phase, MergePhase::Typecheck);
    assert_eq!(event.status, MergePhaseStatus::Started);
    assert_eq!(event.message, "Running type check");
}

#[test]
fn merge_progress_event_sets_timestamp() {
    let before = Utc::now();
    let event = MergeProgressEvent::new(
        "task-123".to_string(),
        MergePhase::Test,
        MergePhaseStatus::Passed,
        "Tests passed".to_string(),
    );
    let after = Utc::now();

    assert!(event.timestamp >= before);
    assert!(event.timestamp <= after);
}

#[test]
fn merge_progress_event_serializes() {
    let event = MergeProgressEvent::new(
        "task-456".to_string(),
        MergePhase::Clippy,
        MergePhaseStatus::Failed,
        "Clippy errors found".to_string(),
    );

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"task_id\":\"task-456\""));
    assert!(json.contains("\"phase\":\"clippy\""));
    assert!(json.contains("\"status\":\"failed\""));
    assert!(json.contains("\"message\":\"Clippy errors found\""));
}

#[test]
fn merge_phase_display() {
    assert_eq!(format!("{}", MergePhase::WorktreeSetup), "worktree_setup");
    assert_eq!(
        format!("{}", MergePhase::ProgrammaticMerge),
        "programmatic_merge"
    );
    assert_eq!(format!("{}", MergePhase::Typecheck), "typecheck");
    assert_eq!(format!("{}", MergePhase::Lint), "lint");
    assert_eq!(format!("{}", MergePhase::Clippy), "clippy");
    assert_eq!(format!("{}", MergePhase::Test), "test");
    assert_eq!(format!("{}", MergePhase::Finalize), "finalize");
}

#[test]
fn merge_phase_status_display() {
    assert_eq!(format!("{}", MergePhaseStatus::Started), "started");
    assert_eq!(format!("{}", MergePhaseStatus::Passed), "passed");
    assert_eq!(format!("{}", MergePhaseStatus::Failed), "failed");
}

#[test]
fn merge_phase_equality() {
    assert_eq!(MergePhase::Typecheck, MergePhase::Typecheck);
    assert_ne!(MergePhase::Typecheck, MergePhase::Lint);
}

#[test]
fn merge_phase_status_equality() {
    assert_eq!(MergePhaseStatus::Started, MergePhaseStatus::Started);
    assert_ne!(MergePhaseStatus::Started, MergePhaseStatus::Passed);
}

// Phase mapper tests
#[test]
fn map_command_npm_typecheck() {
    assert_eq!(
        map_command_to_phase("npm run typecheck"),
        MergePhase::Typecheck
    );
}

#[test]
fn map_command_npx_tsc() {
    assert_eq!(
        map_command_to_phase("npx tsc --noEmit"),
        MergePhase::Typecheck
    );
}

#[test]
fn map_command_npm_lint() {
    assert_eq!(map_command_to_phase("npm run lint"), MergePhase::Lint);
}

#[test]
fn map_command_cargo_clippy() {
    assert_eq!(
        map_command_to_phase("cargo clippy --all-targets --all-features -- -D warnings"),
        MergePhase::Clippy
    );
}

#[test]
fn map_command_cargo_test() {
    assert_eq!(map_command_to_phase("cargo test"), MergePhase::Test);
}

#[test]
fn map_command_npm_test() {
    assert_eq!(map_command_to_phase("npm run test"), MergePhase::Test);
}

#[test]
fn map_command_npm_test_run() {
    assert_eq!(map_command_to_phase("npm run test:run"), MergePhase::Test);
}

#[test]
fn map_command_unknown() {
    assert_eq!(
        map_command_to_phase("some unknown command"),
        MergePhase::Finalize
    );
}

#[test]
fn map_command_case_insensitive() {
    assert_eq!(
        map_command_to_phase("NPM RUN TYPECHECK"),
        MergePhase::Typecheck
    );
    assert_eq!(map_command_to_phase("CARGO CLIPPY"), MergePhase::Clippy);
}

#[test]
fn map_command_with_extra_flags() {
    assert_eq!(
        map_command_to_phase("npm run typecheck -- --strict"),
        MergePhase::Typecheck
    );
    assert_eq!(
        map_command_to_phase("cargo test --lib --release"),
        MergePhase::Test
    );
}
