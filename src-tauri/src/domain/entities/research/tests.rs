// Tests for research entities

use super::*;
use crate::domain::entities::artifact::ArtifactId;
use std::str::FromStr;

    // ===== ResearchDepthPreset Tests =====

    #[test]
    fn research_depth_preset_all_returns_4_presets() {
        let all = ResearchDepthPreset::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn research_depth_preset_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::QuickScan).unwrap(),
            "\"quick-scan\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::DeepDive).unwrap(),
            "\"deep-dive\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::Exhaustive).unwrap(),
            "\"exhaustive\""
        );
    }

    #[test]
    fn research_depth_preset_deserializes() {
        let p: ResearchDepthPreset = serde_json::from_str("\"quick-scan\"").unwrap();
        assert_eq!(p, ResearchDepthPreset::QuickScan);
        let p: ResearchDepthPreset = serde_json::from_str("\"deep-dive\"").unwrap();
        assert_eq!(p, ResearchDepthPreset::DeepDive);
    }

    #[test]
    fn research_depth_preset_from_str() {
        assert_eq!(
            ResearchDepthPreset::from_str("quick-scan").unwrap(),
            ResearchDepthPreset::QuickScan
        );
        assert_eq!(
            ResearchDepthPreset::from_str("standard").unwrap(),
            ResearchDepthPreset::Standard
        );
        assert_eq!(
            ResearchDepthPreset::from_str("deep-dive").unwrap(),
            ResearchDepthPreset::DeepDive
        );
        assert_eq!(
            ResearchDepthPreset::from_str("exhaustive").unwrap(),
            ResearchDepthPreset::Exhaustive
        );
    }

    #[test]
    fn research_depth_preset_from_str_error() {
        let err = ResearchDepthPreset::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn research_depth_preset_display() {
        assert_eq!(ResearchDepthPreset::QuickScan.to_string(), "quick-scan");
        assert_eq!(ResearchDepthPreset::Standard.to_string(), "standard");
        assert_eq!(ResearchDepthPreset::DeepDive.to_string(), "deep-dive");
        assert_eq!(ResearchDepthPreset::Exhaustive.to_string(), "exhaustive");
    }

    #[test]
    fn research_depth_preset_to_custom_depth() {
        let depth = ResearchDepthPreset::QuickScan.to_custom_depth();
        assert_eq!(depth.max_iterations, 10);
        assert_eq!(depth.timeout_hours, 0.5);
        assert_eq!(depth.checkpoint_interval, 5);
    }

    // ===== CustomDepth Tests =====

    #[test]
    fn custom_depth_new_creates_correctly() {
        let depth = CustomDepth::new(100, 4.0, 20);
        assert_eq!(depth.max_iterations, 100);
        assert_eq!(depth.timeout_hours, 4.0);
        assert_eq!(depth.checkpoint_interval, 20);
    }

    #[test]
    fn custom_depth_presets() {
        let quick = CustomDepth::quick_scan();
        assert_eq!(quick.max_iterations, 10);
        assert_eq!(quick.timeout_hours, 0.5);
        assert_eq!(quick.checkpoint_interval, 5);

        let standard = CustomDepth::standard();
        assert_eq!(standard.max_iterations, 50);
        assert_eq!(standard.timeout_hours, 2.0);
        assert_eq!(standard.checkpoint_interval, 10);

        let deep = CustomDepth::deep_dive();
        assert_eq!(deep.max_iterations, 200);
        assert_eq!(deep.timeout_hours, 8.0);
        assert_eq!(deep.checkpoint_interval, 25);

        let exhaustive = CustomDepth::exhaustive();
        assert_eq!(exhaustive.max_iterations, 500);
        assert_eq!(exhaustive.timeout_hours, 24.0);
        assert_eq!(exhaustive.checkpoint_interval, 50);
    }

    #[test]
    fn custom_depth_default_is_standard() {
        let default = CustomDepth::default();
        assert_eq!(default, CustomDepth::standard());
    }

    #[test]
    fn custom_depth_serializes() {
        let depth = CustomDepth::new(100, 4.0, 20);
        let json = serde_json::to_string(&depth).unwrap();
        assert!(json.contains("\"max_iterations\":100"));
        assert!(json.contains("\"timeout_hours\":4.0"));
        assert!(json.contains("\"checkpoint_interval\":20"));
    }

    #[test]
    fn custom_depth_deserializes() {
        let json = r#"{"max_iterations":75,"timeout_hours":3.5,"checkpoint_interval":15}"#;
        let depth: CustomDepth = serde_json::from_str(json).unwrap();
        assert_eq!(depth.max_iterations, 75);
        assert_eq!(depth.timeout_hours, 3.5);
        assert_eq!(depth.checkpoint_interval, 15);
    }

    // ===== RESEARCH_PRESETS Tests =====

    #[test]
    fn research_presets_index_quick_scan() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::QuickScan];
        assert_eq!(depth.max_iterations, 10);
        assert_eq!(depth.timeout_hours, 0.5);
        assert_eq!(depth.checkpoint_interval, 5);
    }

    #[test]
    fn research_presets_index_standard() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::Standard];
        assert_eq!(depth.max_iterations, 50);
        assert_eq!(depth.timeout_hours, 2.0);
        assert_eq!(depth.checkpoint_interval, 10);
    }

    #[test]
    fn research_presets_index_deep_dive() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::DeepDive];
        assert_eq!(depth.max_iterations, 200);
        assert_eq!(depth.timeout_hours, 8.0);
        assert_eq!(depth.checkpoint_interval, 25);
    }

    #[test]
    fn research_presets_index_exhaustive() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::Exhaustive];
        assert_eq!(depth.max_iterations, 500);
        assert_eq!(depth.timeout_hours, 24.0);
        assert_eq!(depth.checkpoint_interval, 50);
    }

    #[test]
    fn research_presets_get_helper() {
        let depth = ResearchPresets::get(&ResearchDepthPreset::Standard);
        assert_eq!(depth.max_iterations, 50);
    }

    // ===== ResearchDepth Tests =====

    #[test]
    fn research_depth_preset_creates_correctly() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::DeepDive);
        assert!(depth.is_preset());
        assert!(!depth.is_custom());
    }

    #[test]
    fn research_depth_custom_creates_correctly() {
        let depth = ResearchDepth::custom(CustomDepth::new(150, 5.0, 30));
        assert!(depth.is_custom());
        assert!(!depth.is_preset());
    }

    #[test]
    fn research_depth_resolve_preset() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::QuickScan);
        let resolved = depth.resolve();
        assert_eq!(resolved.max_iterations, 10);
        assert_eq!(resolved.timeout_hours, 0.5);
    }

    #[test]
    fn research_depth_resolve_custom() {
        let custom = CustomDepth::new(150, 5.0, 30);
        let depth = ResearchDepth::custom(custom);
        let resolved = depth.resolve();
        assert_eq!(resolved.max_iterations, 150);
        assert_eq!(resolved.timeout_hours, 5.0);
    }

    #[test]
    fn research_depth_default_is_standard_preset() {
        let default = ResearchDepth::default();
        assert!(default.is_preset());
        if let ResearchDepth::Preset(p) = default {
            assert_eq!(p, ResearchDepthPreset::Standard);
        } else {
            panic!("Expected preset");
        }
    }

    #[test]
    fn research_depth_serializes_preset() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::DeepDive);
        let json = serde_json::to_string(&depth).unwrap();
        assert_eq!(json, "\"deep-dive\"");
    }

    #[test]
    fn research_depth_serializes_custom() {
        let depth = ResearchDepth::custom(CustomDepth::new(100, 4.0, 20));
        let json = serde_json::to_string(&depth).unwrap();
        assert!(json.contains("\"max_iterations\":100"));
    }

    #[test]
    fn research_depth_deserializes_preset() {
        let json = "\"quick-scan\"";
        let depth: ResearchDepth = serde_json::from_str(json).unwrap();
        assert!(depth.is_preset());
    }

    #[test]
    fn research_depth_deserializes_custom() {
        let json = r#"{"max_iterations":100,"timeout_hours":4.0,"checkpoint_interval":20}"#;
        let depth: ResearchDepth = serde_json::from_str(json).unwrap();
        assert!(depth.is_custom());
    }

    // ===== ResearchProcessStatus Tests =====

    #[test]
    fn research_process_status_all_returns_5_statuses() {
        let all = ResearchProcessStatus::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn research_process_status_serializes() {
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Paused).unwrap(),
            "\"paused\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn research_process_status_deserializes() {
        let s: ResearchProcessStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(s, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_process_status_from_str() {
        assert_eq!(
            ResearchProcessStatus::from_str("pending").unwrap(),
            ResearchProcessStatus::Pending
        );
        assert_eq!(
            ResearchProcessStatus::from_str("running").unwrap(),
            ResearchProcessStatus::Running
        );
        assert_eq!(
            ResearchProcessStatus::from_str("paused").unwrap(),
            ResearchProcessStatus::Paused
        );
        assert_eq!(
            ResearchProcessStatus::from_str("completed").unwrap(),
            ResearchProcessStatus::Completed
        );
        assert_eq!(
            ResearchProcessStatus::from_str("failed").unwrap(),
            ResearchProcessStatus::Failed
        );
    }

    #[test]
    fn research_process_status_from_str_error() {
        let err = ResearchProcessStatus::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn research_process_status_is_active() {
        assert!(ResearchProcessStatus::Pending.is_active());
        assert!(ResearchProcessStatus::Running.is_active());
        assert!(!ResearchProcessStatus::Paused.is_active());
        assert!(!ResearchProcessStatus::Completed.is_active());
        assert!(!ResearchProcessStatus::Failed.is_active());
    }

    #[test]
    fn research_process_status_is_terminal() {
        assert!(!ResearchProcessStatus::Pending.is_terminal());
        assert!(!ResearchProcessStatus::Running.is_terminal());
        assert!(!ResearchProcessStatus::Paused.is_terminal());
        assert!(ResearchProcessStatus::Completed.is_terminal());
        assert!(ResearchProcessStatus::Failed.is_terminal());
    }

    #[test]
    fn research_process_status_display() {
        assert_eq!(ResearchProcessStatus::Running.to_string(), "running");
        assert_eq!(ResearchProcessStatus::Completed.to_string(), "completed");
    }

    // ===== ResearchBrief Tests =====

    #[test]
    fn research_brief_new_creates_with_question() {
        let brief = ResearchBrief::new("What is the best architecture?");
        assert_eq!(brief.question, "What is the best architecture?");
        assert!(brief.context.is_none());
        assert!(brief.scope.is_none());
        assert!(brief.constraints.is_empty());
    }

    #[test]
    fn research_brief_with_context() {
        let brief = ResearchBrief::new("Question")
            .with_context("Context info");
        assert_eq!(brief.context, Some("Context info".to_string()));
    }

    #[test]
    fn research_brief_with_scope() {
        let brief = ResearchBrief::new("Question")
            .with_scope("Backend only");
        assert_eq!(brief.scope, Some("Backend only".to_string()));
    }

    #[test]
    fn research_brief_with_constraint() {
        let brief = ResearchBrief::new("Question")
            .with_constraint("Must be fast")
            .with_constraint("Must be secure");
        assert_eq!(brief.constraints.len(), 2);
        assert!(brief.constraints.contains(&"Must be fast".to_string()));
        assert!(brief.constraints.contains(&"Must be secure".to_string()));
    }

    #[test]
    fn research_brief_with_constraints() {
        let brief = ResearchBrief::new("Question")
            .with_constraints(["Constraint 1", "Constraint 2"]);
        assert_eq!(brief.constraints.len(), 2);
    }

    #[test]
    fn research_brief_serializes() {
        let brief = ResearchBrief::new("Question")
            .with_context("Context")
            .with_constraint("Constraint");
        let json = serde_json::to_string(&brief).unwrap();
        assert!(json.contains("\"question\":\"Question\""));
        assert!(json.contains("\"context\":\"Context\""));
        assert!(json.contains("\"constraints\":[\"Constraint\"]"));
    }

    #[test]
    fn research_brief_deserializes() {
        let json = r#"{"question":"Test question","context":"Test context"}"#;
        let brief: ResearchBrief = serde_json::from_str(json).unwrap();
        assert_eq!(brief.question, "Test question");
        assert_eq!(brief.context, Some("Test context".to_string()));
    }

    // ===== ResearchOutput Tests =====

    #[test]
    fn research_output_new_creates_correctly() {
        let output = ResearchOutput::new("my-bucket");
        assert_eq!(output.target_bucket, "my-bucket");
        assert!(output.artifact_types.is_empty());
    }

    #[test]
    fn research_output_with_artifact_type() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Recommendations);
        assert_eq!(output.artifact_types.len(), 2);
    }

    #[test]
    fn research_output_with_artifact_type_no_duplicates() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Findings);
        assert_eq!(output.artifact_types.len(), 1);
    }

    #[test]
    fn research_output_default_has_research_outputs_bucket() {
        let output = ResearchOutput::default();
        assert_eq!(output.target_bucket, "research-outputs");
        assert!(output.artifact_types.contains(&ArtifactType::ResearchDocument));
        assert!(output.artifact_types.contains(&ArtifactType::Findings));
        assert!(output.artifact_types.contains(&ArtifactType::Recommendations));
    }

    #[test]
    fn research_output_serializes() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings);
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"target_bucket\":\"bucket\""));
        assert!(json.contains("\"findings\""));
    }

    // ===== ResearchProgress Tests =====

    #[test]
    fn research_progress_new_is_pending() {
        let progress = ResearchProgress::new();
        assert_eq!(progress.current_iteration, 0);
        assert_eq!(progress.status, ResearchProcessStatus::Pending);
        assert!(progress.last_checkpoint.is_none());
        assert!(progress.error_message.is_none());
    }

    #[test]
    fn research_progress_start() {
        let mut progress = ResearchProgress::new();
        progress.start();
        assert_eq!(progress.status, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_progress_advance() {
        let mut progress = ResearchProgress::new();
        progress.advance();
        assert_eq!(progress.current_iteration, 1);
        progress.advance();
        assert_eq!(progress.current_iteration, 2);
    }

    #[test]
    fn research_progress_pause_resume() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.pause();
        assert_eq!(progress.status, ResearchProcessStatus::Paused);
        progress.resume();
        assert_eq!(progress.status, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_progress_complete() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.complete();
        assert_eq!(progress.status, ResearchProcessStatus::Completed);
    }

    #[test]
    fn research_progress_fail() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.fail("Something went wrong");
        assert_eq!(progress.status, ResearchProcessStatus::Failed);
        assert_eq!(progress.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn research_progress_checkpoint() {
        let mut progress = ResearchProgress::new();
        let artifact_id = ArtifactId::from_string("checkpoint-1");
        progress.checkpoint(artifact_id.clone());
        assert_eq!(progress.last_checkpoint, Some(artifact_id));
    }

    #[test]
    fn research_progress_percentage() {
        let mut progress = ResearchProgress::new();
        assert_eq!(progress.percentage(100), 0.0);
        progress.current_iteration = 25;
        assert_eq!(progress.percentage(100), 25.0);
        progress.current_iteration = 50;
        assert_eq!(progress.percentage(100), 50.0);
        progress.current_iteration = 100;
        assert_eq!(progress.percentage(100), 100.0);
    }

    #[test]
    fn research_progress_percentage_over_max() {
        let mut progress = ResearchProgress::new();
        progress.current_iteration = 150;
        assert_eq!(progress.percentage(100), 100.0);
    }

    #[test]
    fn research_progress_percentage_zero_max() {
        let progress = ResearchProgress::new();
        assert_eq!(progress.percentage(0), 0.0);
    }

    #[test]
    fn research_progress_serializes() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.current_iteration = 10;
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"current_iteration\":10"));
        assert!(json.contains("\"status\":\"running\""));
    }

    // ===== ResearchProcess Tests =====

    #[test]
    fn research_process_new_creates_correctly() {
        let brief = ResearchBrief::new("What framework to use?");
        let process = ResearchProcess::new("Framework Research", brief, "deep-researcher");
        assert_eq!(process.name, "Framework Research");
        assert_eq!(process.agent_profile_id, "deep-researcher");
        assert_eq!(process.brief.question, "What framework to use?");
        assert!(process.depth.is_preset());
        assert_eq!(process.progress.status, ResearchProcessStatus::Pending);
        assert!(process.started_at.is_none());
        assert!(process.completed_at.is_none());
    }

    #[test]
    fn research_process_with_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_depth(ResearchDepth::preset(ResearchDepthPreset::DeepDive));
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 200);
    }

    #[test]
    fn research_process_with_preset() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::Exhaustive);
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 500);
    }

    #[test]
    fn research_process_with_custom_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
    }

    #[test]
    fn research_process_with_output() {
        let brief = ResearchBrief::new("Question");
        let output = ResearchOutput::new("custom-bucket")
            .with_artifact_type(ArtifactType::Findings);
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_output(output);
        assert_eq!(process.output.target_bucket, "custom-bucket");
    }

    #[test]
    fn research_process_lifecycle() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan);

        // Initial state
        assert_eq!(process.status(), ResearchProcessStatus::Pending);
        assert!(process.started_at.is_none());

        // Start
        process.start();
        assert_eq!(process.status(), ResearchProcessStatus::Running);
        assert!(process.started_at.is_some());
        assert!(process.is_active());

        // Advance
        process.advance();
        assert_eq!(process.progress.current_iteration, 1);

        // Pause
        process.pause();
        assert_eq!(process.status(), ResearchProcessStatus::Paused);
        assert!(!process.is_active());

        // Resume
        process.resume();
        assert_eq!(process.status(), ResearchProcessStatus::Running);

        // Complete
        process.complete();
        assert_eq!(process.status(), ResearchProcessStatus::Completed);
        assert!(process.is_terminal());
        assert!(process.completed_at.is_some());
    }

    #[test]
    fn research_process_fail() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.fail("Network error");
        assert_eq!(process.status(), ResearchProcessStatus::Failed);
        assert!(process.is_terminal());
        assert_eq!(
            process.progress.error_message,
            Some("Network error".to_string())
        );
    }

    #[test]
    fn research_process_checkpoint() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent");
        let artifact_id = ArtifactId::from_string("checkpoint-artifact");
        process.checkpoint(artifact_id.clone());
        assert_eq!(process.progress.last_checkpoint, Some(artifact_id));
    }

    #[test]
    fn research_process_progress_percentage() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // 10 max iterations
        assert_eq!(process.progress_percentage(), 0.0);
        process.progress.current_iteration = 5;
        assert_eq!(process.progress_percentage(), 50.0);
        process.progress.current_iteration = 10;
        assert_eq!(process.progress_percentage(), 100.0);
    }

    #[test]
    fn research_process_should_checkpoint() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // checkpoint_interval = 5

        process.progress.current_iteration = 0;
        assert!(!process.should_checkpoint());

        process.progress.current_iteration = 4;
        assert!(!process.should_checkpoint());

        process.progress.current_iteration = 5;
        assert!(process.should_checkpoint());

        process.progress.current_iteration = 10;
        assert!(process.should_checkpoint());

        process.progress.current_iteration = 7;
        assert!(!process.should_checkpoint());
    }

    #[test]
    fn research_process_is_max_iterations_reached() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // 10 max iterations

        process.progress.current_iteration = 5;
        assert!(!process.is_max_iterations_reached());

        process.progress.current_iteration = 10;
        assert!(process.is_max_iterations_reached());

        process.progress.current_iteration = 15;
        assert!(process.is_max_iterations_reached());
    }

    #[test]
    fn research_process_serializes_roundtrip() {
        let brief = ResearchBrief::new("What is the best approach?")
            .with_context("Building a new feature")
            .with_constraint("Must be maintainable");
        let process = ResearchProcess::new("Architecture Research", brief, "deep-researcher")
            .with_preset(ResearchDepthPreset::DeepDive);

        let json = serde_json::to_string(&process).unwrap();
        let parsed: ResearchProcess = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "Architecture Research");
        assert_eq!(parsed.agent_profile_id, "deep-researcher");
        assert_eq!(parsed.brief.question, "What is the best approach?");
        assert!(parsed.depth.is_preset());
    }

    #[test]
    fn research_process_serializes_with_custom_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));

        let json = serde_json::to_string(&process).unwrap();
        let parsed: ResearchProcess = serde_json::from_str(&json).unwrap();

        assert!(parsed.depth.is_custom());
        let resolved = parsed.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
    }
