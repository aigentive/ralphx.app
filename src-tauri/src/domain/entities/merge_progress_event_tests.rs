use super::*;

#[test]
fn merge_phase_serializes_as_string() {
    assert_eq!(
        serde_json::to_string(&MergePhase::worktree_setup()).unwrap(),
        "\"worktree_setup\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::programmatic_merge()).unwrap(),
        "\"programmatic_merge\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::finalize()).unwrap(),
        "\"finalize\""
    );
    assert_eq!(
        serde_json::to_string(&MergePhase::new("npm_run_typecheck")).unwrap(),
        "\"npm_run_typecheck\""
    );
}

#[test]
fn merge_phase_deserializes_from_string() {
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"worktree_setup\"").unwrap(),
        MergePhase::worktree_setup()
    );
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"programmatic_merge\"").unwrap(),
        MergePhase::programmatic_merge()
    );
    assert_eq!(
        serde_json::from_str::<MergePhase>("\"cargo_clippy\"").unwrap(),
        MergePhase::new("cargo_clippy")
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
    assert_eq!(
        serde_json::to_string(&MergePhaseStatus::Skipped).unwrap(),
        "\"skipped\""
    );
}

#[test]
fn merge_progress_event_new_sets_fields() {
    let event = MergeProgressEvent::new(
        "task-123".to_string(),
        MergePhase::new("npm_run_typecheck"),
        MergePhaseStatus::Started,
        "Running type check".to_string(),
    );

    assert_eq!(event.task_id, "task-123");
    assert_eq!(event.phase, MergePhase::new("npm_run_typecheck"));
    assert_eq!(event.status, MergePhaseStatus::Started);
    assert_eq!(event.message, "Running type check");
}

#[test]
fn merge_progress_event_sets_timestamp() {
    let before = Utc::now();
    let event = MergeProgressEvent::new(
        "task-123".to_string(),
        MergePhase::new("cargo_test"),
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
        MergePhase::new("cargo_clippy"),
        MergePhaseStatus::Failed,
        "Clippy errors found".to_string(),
    );

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"task_id\":\"task-456\""));
    assert!(json.contains("\"phase\":\"cargo_clippy\""));
    assert!(json.contains("\"status\":\"failed\""));
    assert!(json.contains("\"message\":\"Clippy errors found\""));
}

#[test]
fn merge_phase_display() {
    assert_eq!(
        format!("{}", MergePhase::worktree_setup()),
        "worktree_setup"
    );
    assert_eq!(
        format!("{}", MergePhase::programmatic_merge()),
        "programmatic_merge"
    );
    assert_eq!(format!("{}", MergePhase::finalize()), "finalize");
    assert_eq!(format!("{}", MergePhase::new("cargo_test")), "cargo_test");
}

#[test]
fn merge_phase_status_display() {
    assert_eq!(format!("{}", MergePhaseStatus::Started), "started");
    assert_eq!(format!("{}", MergePhaseStatus::Passed), "passed");
    assert_eq!(format!("{}", MergePhaseStatus::Failed), "failed");
    assert_eq!(format!("{}", MergePhaseStatus::Skipped), "skipped");
}

#[test]
fn merge_phase_equality() {
    assert_eq!(
        MergePhase::new("npm_run_typecheck"),
        MergePhase::new("npm_run_typecheck")
    );
    assert_ne!(
        MergePhase::new("npm_run_typecheck"),
        MergePhase::new("npm_run_lint")
    );
}

#[test]
fn merge_phase_status_equality() {
    assert_eq!(MergePhaseStatus::Started, MergePhaseStatus::Started);
    assert_ne!(MergePhaseStatus::Started, MergePhaseStatus::Passed);
}

// derive_phase_id tests
#[test]
fn derive_phase_id_npm_typecheck() {
    assert_eq!(derive_phase_id("npm run typecheck"), "npm_run_typecheck");
}

#[test]
fn derive_phase_id_npx_tsc() {
    assert_eq!(derive_phase_id("npx tsc --noEmit"), "npx_tsc");
}

#[test]
fn derive_phase_id_npm_lint() {
    assert_eq!(derive_phase_id("npm run lint"), "npm_run_lint");
}

#[test]
fn derive_phase_id_cargo_clippy() {
    assert_eq!(
        derive_phase_id("cargo clippy --all-targets --all-features -- -D warnings"),
        "cargo_clippy"
    );
}

#[test]
fn derive_phase_id_cargo_test() {
    assert_eq!(derive_phase_id("cargo test"), "cargo_test");
}

#[test]
fn derive_phase_id_npm_test() {
    assert_eq!(derive_phase_id("npm run test"), "npm_run_test");
}

#[test]
fn derive_phase_id_npm_test_run() {
    assert_eq!(derive_phase_id("npm run test:run"), "npm_run_test:run");
}

#[test]
fn derive_phase_id_unknown() {
    assert_eq!(
        derive_phase_id("some unknown command"),
        "some_unknown_command"
    );
}

#[test]
fn derive_phase_id_empty() {
    assert_eq!(derive_phase_id(""), "unknown");
}

#[test]
fn derive_phase_id_only_flags() {
    assert_eq!(derive_phase_id("--flag -v"), "unknown");
}

// derive_phase_label tests
#[test]
fn derive_phase_label_npm_typecheck() {
    assert_eq!(derive_phase_label("npm run typecheck"), "Type Check");
}

#[test]
fn derive_phase_label_npx_tsc() {
    assert_eq!(derive_phase_label("npx tsc --noEmit"), "Type Check");
}

#[test]
fn derive_phase_label_npm_lint() {
    assert_eq!(derive_phase_label("npm run lint"), "Lint");
}

#[test]
fn derive_phase_label_cargo_clippy() {
    assert_eq!(
        derive_phase_label("cargo clippy --all-targets --all-features -- -D warnings"),
        "Clippy"
    );
}

#[test]
fn derive_phase_label_cargo_test() {
    assert_eq!(derive_phase_label("cargo test"), "Test");
}

#[test]
fn derive_phase_label_npm_test() {
    assert_eq!(derive_phase_label("npm run test"), "Test");
}

#[test]
fn derive_phase_label_unknown_fallback() {
    // mypy is a known type checker
    assert_eq!(derive_phase_label("mypy src/"), "Type Check");
}

#[test]
fn derive_phase_label_truly_unknown() {
    // Unknown commands get title-cased phase ID
    assert_eq!(derive_phase_label("custom-tool run"), "Custom-tool Run");
}

#[test]
fn derive_phase_label_go_test() {
    assert_eq!(derive_phase_label("go test ./..."), "Test");
}

#[test]
fn derive_phase_label_ruff_lint() {
    assert_eq!(derive_phase_label("ruff check ."), "Lint");
}

#[test]
fn derive_phase_label_eslint() {
    assert_eq!(derive_phase_label("eslint src/"), "Lint");
}

#[test]
fn derive_phase_label_cargo_fmt() {
    assert_eq!(derive_phase_label("cargo fmt --check"), "Format");
}

// map_command_to_phase tests (new behavior: returns MergePhase with dynamic ID)
#[test]
fn map_command_npm_typecheck() {
    assert_eq!(
        map_command_to_phase("npm run typecheck"),
        MergePhase::new("npm_run_typecheck")
    );
}

#[test]
fn map_command_cargo_clippy() {
    assert_eq!(
        map_command_to_phase("cargo clippy --all-targets --all-features -- -D warnings"),
        MergePhase::new("cargo_clippy")
    );
}

#[test]
fn map_command_cargo_test() {
    assert_eq!(
        map_command_to_phase("cargo test"),
        MergePhase::new("cargo_test")
    );
}

// derive_phases_from_analysis tests
#[test]
fn derive_phases_from_analysis_basic() {
    let entries = vec![
        PhaseAnalysisEntry {
            validate: vec!["npm run typecheck".to_string(), "npm run lint".to_string()],
        },
        PhaseAnalysisEntry {
            validate: vec![
                "cargo clippy --all-targets --all-features -- -D warnings".to_string(),
                "cargo test".to_string(),
            ],
        },
    ];

    let phases = derive_phases_from_analysis(&entries);

    // 4 pre-merge + worktree_setup + programmatic_merge + 4 validate + finalize = 11
    assert_eq!(phases.len(), 11);
    assert_eq!(phases[0].id, "merge_preparation");
    assert_eq!(phases[1].id, "precondition_check");
    assert_eq!(phases[2].id, "branch_freshness");
    assert_eq!(phases[3].id, "merge_cleanup");
    assert_eq!(phases[4].id, "worktree_setup");
    assert_eq!(phases[4].label, "Worktree Setup");
    assert_eq!(phases[5].id, "programmatic_merge");
    assert_eq!(phases[5].label, "Merge");
    assert_eq!(phases[6].id, "npm_run_typecheck");
    assert_eq!(phases[6].label, "Type Check");
    assert_eq!(phases[7].id, "npm_run_lint");
    assert_eq!(phases[7].label, "Lint");
    assert_eq!(phases[8].id, "cargo_clippy");
    assert_eq!(phases[8].label, "Clippy");
    assert_eq!(phases[9].id, "cargo_test");
    assert_eq!(phases[9].label, "Test");
    assert_eq!(phases[10].id, "finalize");
    assert_eq!(phases[10].label, "Finalize");
}

#[test]
fn derive_phases_from_analysis_empty() {
    let entries: Vec<PhaseAnalysisEntry> = vec![];
    let phases = derive_phases_from_analysis(&entries);

    // 4 pre-merge + structural phases (worktree_setup, programmatic_merge, finalize) = 7
    assert_eq!(phases.len(), 7);
    assert_eq!(phases[0].id, "merge_preparation");
    assert_eq!(phases[1].id, "precondition_check");
    assert_eq!(phases[2].id, "branch_freshness");
    assert_eq!(phases[3].id, "merge_cleanup");
    assert_eq!(phases[4].id, "worktree_setup");
    assert_eq!(phases[5].id, "programmatic_merge");
    assert_eq!(phases[6].id, "finalize");
}

#[test]
fn derive_phases_from_analysis_single_entry() {
    let entries = vec![PhaseAnalysisEntry {
        validate: vec!["npx tsc --noEmit".to_string()],
    }];

    let phases = derive_phases_from_analysis(&entries);
    // 4 pre-merge + worktree_setup + programmatic_merge + 1 validate + finalize = 8
    assert_eq!(phases.len(), 8);
    assert_eq!(phases[6].id, "npx_tsc");
    assert_eq!(phases[6].label, "Type Check");
}

#[test]
fn merge_phase_info_serializes() {
    let info = MergePhaseInfo {
        id: "cargo_test".to_string(),
        label: "Test".to_string(),
        command: Some("cargo test".to_string()),
        description: None,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"id\":\"cargo_test\""));
    assert!(json.contains("\"label\":\"Test\""));
    assert!(json.contains("\"command\":\"cargo test\""));
    // description is None → should be skipped
    assert!(!json.contains("\"description\""));
}
