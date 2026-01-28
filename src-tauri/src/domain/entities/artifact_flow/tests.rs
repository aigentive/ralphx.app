use super::types::*;
use super::*;
use crate::domain::entities::artifact::{
    Artifact, ArtifactContent, ArtifactId, ArtifactMetadata, ArtifactType,
};
use std::str::FromStr;

// ===== ArtifactFlowId Tests =====

#[test]
fn artifact_flow_id_new_generates_valid_uuid() {
    let id = ArtifactFlowId::new();
    assert_eq!(id.as_str().len(), 36);
    assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
}

#[test]
fn artifact_flow_id_from_string_preserves_value() {
    let id = ArtifactFlowId::from_string("flow-123");
    assert_eq!(id.as_str(), "flow-123");
}

#[test]
fn artifact_flow_id_equality_works() {
    let id1 = ArtifactFlowId::from_string("f1");
    let id2 = ArtifactFlowId::from_string("f1");
    let id3 = ArtifactFlowId::from_string("f2");
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn artifact_flow_id_serializes() {
    let id = ArtifactFlowId::from_string("serialize-test");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"serialize-test\"");
}

#[test]
fn artifact_flow_id_deserializes() {
    let json = "\"deserialize-test\"";
    let id: ArtifactFlowId = serde_json::from_str(json).unwrap();
    assert_eq!(id.as_str(), "deserialize-test");
}

// ===== ArtifactFlowEvent Tests =====

#[test]
fn artifact_flow_event_all_returns_4_events() {
    let all = ArtifactFlowEvent::all();
    assert_eq!(all.len(), 4);
}

#[test]
fn artifact_flow_event_serializes_snake_case() {
    assert_eq!(
        serde_json::to_string(&ArtifactFlowEvent::ArtifactCreated).unwrap(),
        "\"artifact_created\""
    );
    assert_eq!(
        serde_json::to_string(&ArtifactFlowEvent::TaskCompleted).unwrap(),
        "\"task_completed\""
    );
    assert_eq!(
        serde_json::to_string(&ArtifactFlowEvent::ProcessCompleted).unwrap(),
        "\"process_completed\""
    );
}

#[test]
fn artifact_flow_event_deserializes() {
    let e: ArtifactFlowEvent = serde_json::from_str("\"artifact_created\"").unwrap();
    assert_eq!(e, ArtifactFlowEvent::ArtifactCreated);
    let e: ArtifactFlowEvent = serde_json::from_str("\"task_completed\"").unwrap();
    assert_eq!(e, ArtifactFlowEvent::TaskCompleted);
}

#[test]
fn artifact_flow_event_from_str() {
    assert_eq!(
        ArtifactFlowEvent::from_str("artifact_created").unwrap(),
        ArtifactFlowEvent::ArtifactCreated
    );
    assert_eq!(
        ArtifactFlowEvent::from_str("task_completed").unwrap(),
        ArtifactFlowEvent::TaskCompleted
    );
    assert_eq!(
        ArtifactFlowEvent::from_str("process_completed").unwrap(),
        ArtifactFlowEvent::ProcessCompleted
    );
}

#[test]
fn artifact_flow_event_from_str_error() {
    let err = ArtifactFlowEvent::from_str("invalid").unwrap_err();
    assert_eq!(err.value, "invalid");
    assert!(err.to_string().contains("invalid"));
}

#[test]
fn artifact_flow_event_display() {
    assert_eq!(
        ArtifactFlowEvent::ArtifactCreated.to_string(),
        "artifact_created"
    );
    assert_eq!(
        ArtifactFlowEvent::TaskCompleted.to_string(),
        "task_completed"
    );
}

// ===== ArtifactFlowFilter Tests =====

fn create_test_artifact(
    artifact_type: ArtifactType,
    bucket_id: Option<&str>,
) -> Artifact {
    use crate::domain::entities::artifact::ArtifactBucketId;
    Artifact {
        id: ArtifactId::from_string("test-artifact"),
        artifact_type,
        name: "Test Artifact".to_string(),
        content: ArtifactContent::inline("Test content"),
        metadata: ArtifactMetadata::new("user"),
        derived_from: vec![],
        bucket_id: bucket_id.map(ArtifactBucketId::from_string),
    }
}

#[test]
fn artifact_flow_filter_empty_matches_all() {
    let filter = ArtifactFlowFilter::new();
    assert!(filter.is_empty());
    let artifact = create_test_artifact(ArtifactType::Prd, Some("bucket-1"));
    assert!(filter.matches(&artifact));
}

#[test]
fn artifact_flow_filter_artifact_types_matches() {
    let filter =
        ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd, ArtifactType::DesignDoc]);
    let prd = create_test_artifact(ArtifactType::Prd, None);
    let design = create_test_artifact(ArtifactType::DesignDoc, None);
    let code = create_test_artifact(ArtifactType::CodeChange, None);
    assert!(filter.matches(&prd));
    assert!(filter.matches(&design));
    assert!(!filter.matches(&code));
}

#[test]
fn artifact_flow_filter_source_bucket_matches() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let filter = ArtifactFlowFilter::new()
        .with_source_bucket(ArtifactBucketId::from_string("research-outputs"));
    let in_bucket = create_test_artifact(ArtifactType::Findings, Some("research-outputs"));
    let other_bucket = create_test_artifact(ArtifactType::Findings, Some("other-bucket"));
    let no_bucket = create_test_artifact(ArtifactType::Findings, None);
    assert!(filter.matches(&in_bucket));
    assert!(!filter.matches(&other_bucket));
    assert!(!filter.matches(&no_bucket));
}

#[test]
fn artifact_flow_filter_combined_matches() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let filter = ArtifactFlowFilter::new()
        .with_artifact_types(vec![ArtifactType::Recommendations])
        .with_source_bucket(ArtifactBucketId::from_string("research-outputs"));
    // Matches both
    let good = create_test_artifact(ArtifactType::Recommendations, Some("research-outputs"));
    assert!(filter.matches(&good));
    // Wrong type
    let wrong_type = create_test_artifact(ArtifactType::Findings, Some("research-outputs"));
    assert!(!filter.matches(&wrong_type));
    // Wrong bucket
    let wrong_bucket = create_test_artifact(ArtifactType::Recommendations, Some("other"));
    assert!(!filter.matches(&wrong_bucket));
}

#[test]
fn artifact_flow_filter_serializes() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let filter = ArtifactFlowFilter::new()
        .with_artifact_types(vec![ArtifactType::Prd])
        .with_source_bucket(ArtifactBucketId::from_string("bucket-1"));
    let json = serde_json::to_string(&filter).unwrap();
    assert!(json.contains("\"artifact_types\""));
    assert!(json.contains("\"prd\""));
    assert!(json.contains("\"source_bucket\""));
}

#[test]
fn artifact_flow_filter_deserializes() {
    let json = r#"{"artifact_types":["prd","design_doc"],"source_bucket":"bucket-1"}"#;
    let filter: ArtifactFlowFilter = serde_json::from_str(json).unwrap();
    assert_eq!(filter.artifact_types.unwrap().len(), 2);
    assert_eq!(filter.source_bucket.unwrap().as_str(), "bucket-1");
}

// ===== ArtifactFlowTrigger Tests =====

#[test]
fn artifact_flow_trigger_on_event_creates_correctly() {
    let trigger = ArtifactFlowTrigger::on_event(ArtifactFlowEvent::ArtifactCreated);
    assert_eq!(trigger.event, ArtifactFlowEvent::ArtifactCreated);
    assert!(trigger.filter.is_none());
}

#[test]
fn artifact_flow_trigger_convenience_constructors() {
    let t1 = ArtifactFlowTrigger::on_artifact_created();
    assert_eq!(t1.event, ArtifactFlowEvent::ArtifactCreated);
    let t2 = ArtifactFlowTrigger::on_task_completed();
    assert_eq!(t2.event, ArtifactFlowEvent::TaskCompleted);
    let t3 = ArtifactFlowTrigger::on_process_completed();
    assert_eq!(t3.event, ArtifactFlowEvent::ProcessCompleted);
}

#[test]
fn artifact_flow_trigger_with_filter() {
    let trigger = ArtifactFlowTrigger::on_artifact_created()
        .with_filter(ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd]));
    assert!(trigger.filter.is_some());
}

#[test]
fn artifact_flow_trigger_matches_artifact_no_filter() {
    let trigger = ArtifactFlowTrigger::on_artifact_created();
    let artifact = create_test_artifact(ArtifactType::Prd, None);
    assert!(trigger.matches_artifact(&artifact));
}

#[test]
fn artifact_flow_trigger_matches_artifact_with_filter() {
    let trigger = ArtifactFlowTrigger::on_artifact_created()
        .with_filter(ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd]));
    let prd = create_test_artifact(ArtifactType::Prd, None);
    let code = create_test_artifact(ArtifactType::CodeChange, None);
    assert!(trigger.matches_artifact(&prd));
    assert!(!trigger.matches_artifact(&code));
}

#[test]
fn artifact_flow_trigger_serializes() {
    let trigger = ArtifactFlowTrigger::on_artifact_created();
    let json = serde_json::to_string(&trigger).unwrap();
    assert!(json.contains("\"event\":\"artifact_created\""));
}

// ===== ArtifactFlowStep Tests =====

#[test]
fn artifact_flow_step_copy_creates_correctly() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let step = ArtifactFlowStep::copy(ArtifactBucketId::from_string("target"));
    assert!(step.is_copy());
    assert!(!step.is_spawn_process());
    assert_eq!(step.step_type(), "copy");
}

#[test]
fn artifact_flow_step_spawn_process_creates_correctly() {
    let step = ArtifactFlowStep::spawn_process("task_decomposition", "orchestrator");
    assert!(step.is_spawn_process());
    assert!(!step.is_copy());
    assert_eq!(step.step_type(), "spawn_process");
}

#[test]
fn artifact_flow_step_copy_serializes() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let step = ArtifactFlowStep::copy(ArtifactBucketId::from_string("bucket-1"));
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"type\":\"copy\""));
    assert!(json.contains("\"to_bucket\":\"bucket-1\""));
}

#[test]
fn artifact_flow_step_spawn_process_serializes() {
    let step = ArtifactFlowStep::spawn_process("task_decomposition", "orchestrator");
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"type\":\"spawn_process\""));
    assert!(json.contains("\"process_type\":\"task_decomposition\""));
    assert!(json.contains("\"agent_profile\":\"orchestrator\""));
}

#[test]
fn artifact_flow_step_deserializes_copy() {
    let json = r#"{"type":"copy","to_bucket":"bucket-1"}"#;
    let step: ArtifactFlowStep = serde_json::from_str(json).unwrap();
    assert!(step.is_copy());
    if let ArtifactFlowStep::Copy { to_bucket } = step {
        assert_eq!(to_bucket.as_str(), "bucket-1");
    } else {
        panic!("Expected copy step");
    }
}

#[test]
fn artifact_flow_step_deserializes_spawn_process() {
    let json = r#"{"type":"spawn_process","process_type":"research","agent_profile":"deep-researcher"}"#;
    let step: ArtifactFlowStep = serde_json::from_str(json).unwrap();
    assert!(step.is_spawn_process());
    if let ArtifactFlowStep::SpawnProcess {
        process_type,
        agent_profile,
    } = step
    {
        assert_eq!(process_type, "research");
        assert_eq!(agent_profile, "deep-researcher");
    } else {
        panic!("Expected spawn_process step");
    }
}

// ===== ArtifactFlow Tests =====

#[test]
fn artifact_flow_new_creates_correctly() {
    let flow =
        ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created());
    assert_eq!(flow.name, "Test Flow");
    assert!(flow.is_active);
    assert!(flow.steps.is_empty());
}

#[test]
fn artifact_flow_with_step() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created())
        .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("target")));
    assert_eq!(flow.steps.len(), 1);
}

#[test]
fn artifact_flow_with_steps() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created())
        .with_steps([
            ArtifactFlowStep::copy(ArtifactBucketId::from_string("target")),
            ArtifactFlowStep::spawn_process("task_decomposition", "orchestrator"),
        ]);
    assert_eq!(flow.steps.len(), 2);
}

#[test]
fn artifact_flow_set_active() {
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created())
        .set_active(false);
    assert!(!flow.is_active);
}

#[test]
fn artifact_flow_should_trigger_when_active_and_matches() {
    let flow = ArtifactFlow::new(
        "Test",
        ArtifactFlowTrigger::on_artifact_created()
            .with_filter(ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd])),
    );
    let prd = create_test_artifact(ArtifactType::Prd, None);
    let code = create_test_artifact(ArtifactType::CodeChange, None);
    assert!(flow.should_trigger(ArtifactFlowEvent::ArtifactCreated, &prd));
    assert!(!flow.should_trigger(ArtifactFlowEvent::ArtifactCreated, &code));
    assert!(!flow.should_trigger(ArtifactFlowEvent::TaskCompleted, &prd));
}

#[test]
fn artifact_flow_should_not_trigger_when_inactive() {
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created())
        .set_active(false);
    let artifact = create_test_artifact(ArtifactType::Prd, None);
    assert!(!flow.should_trigger(ArtifactFlowEvent::ArtifactCreated, &artifact));
}

#[test]
fn artifact_flow_serializes_roundtrip() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let flow = ArtifactFlow::new(
        "Test Flow",
        ArtifactFlowTrigger::on_artifact_created()
            .with_filter(ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd])),
    )
    .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("target")));
    let json = serde_json::to_string(&flow).unwrap();
    let parsed: ArtifactFlow = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, flow.name);
    assert_eq!(parsed.steps.len(), 1);
    assert!(parsed.is_active);
}

// ===== ArtifactFlowContext Tests =====

#[test]
fn artifact_flow_context_artifact_created() {
    let artifact = create_test_artifact(ArtifactType::Prd, None);
    let context = ArtifactFlowContext::artifact_created(artifact.clone());
    assert_eq!(context.event, ArtifactFlowEvent::ArtifactCreated);
    assert!(context.artifact.is_some());
    assert!(context.task_id.is_none());
    assert!(context.process_id.is_none());
}

#[test]
fn artifact_flow_context_task_completed() {
    let artifact = create_test_artifact(ArtifactType::CodeChange, None);
    let context = ArtifactFlowContext::task_completed("task-1", Some(artifact));
    assert_eq!(context.event, ArtifactFlowEvent::TaskCompleted);
    assert!(context.artifact.is_some());
    assert_eq!(context.task_id, Some("task-1".to_string()));
    assert!(context.process_id.is_none());
}

#[test]
fn artifact_flow_context_process_completed() {
    let context = ArtifactFlowContext::process_completed("process-1", None);
    assert_eq!(context.event, ArtifactFlowEvent::ProcessCompleted);
    assert!(context.artifact.is_none());
    assert!(context.task_id.is_none());
    assert_eq!(context.process_id, Some("process-1".to_string()));
}

// ===== ArtifactFlowEngine Tests =====

#[test]
fn artifact_flow_engine_new_is_empty() {
    let engine = ArtifactFlowEngine::new();
    assert_eq!(engine.flow_count(), 0);
    assert!(engine.flows().is_empty());
}

#[test]
fn artifact_flow_engine_register_flow() {
    let mut engine = ArtifactFlowEngine::new();
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created());
    engine.register_flow(flow);
    assert_eq!(engine.flow_count(), 1);
}

#[test]
fn artifact_flow_engine_register_flows() {
    let mut engine = ArtifactFlowEngine::new();
    let flow1 = ArtifactFlow::new("Flow 1", ArtifactFlowTrigger::on_artifact_created());
    let flow2 = ArtifactFlow::new("Flow 2", ArtifactFlowTrigger::on_task_completed());
    engine.register_flows([flow1, flow2]);
    assert_eq!(engine.flow_count(), 2);
}

#[test]
fn artifact_flow_engine_unregister_flow() {
    let mut engine = ArtifactFlowEngine::new();
    let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created());
    let flow_id = flow.id.clone();
    engine.register_flow(flow);
    assert_eq!(engine.flow_count(), 1);
    let removed = engine.unregister_flow(&flow_id);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "Test");
    assert_eq!(engine.flow_count(), 0);
}

#[test]
fn artifact_flow_engine_unregister_nonexistent_returns_none() {
    let mut engine = ArtifactFlowEngine::new();
    let result = engine.unregister_flow(&ArtifactFlowId::from_string("nonexistent"));
    assert!(result.is_none());
}

#[test]
fn artifact_flow_engine_evaluate_triggers_matches_event() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Artifact Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("target"))),
    );
    engine.register_flow(
        ArtifactFlow::new("Task Flow", ArtifactFlowTrigger::on_task_completed())
            .with_step(ArtifactFlowStep::spawn_process("cleanup", "system")),
    );

    let artifact = create_test_artifact(ArtifactType::Prd, None);
    let context = ArtifactFlowContext::artifact_created(artifact);
    let evals = engine.evaluate_triggers(&context);

    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].flow_name, "Artifact Flow");
    assert_eq!(evals[0].steps.len(), 1);
}

#[test]
fn artifact_flow_engine_evaluate_triggers_with_filter() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new(
            "PRD Flow",
            ArtifactFlowTrigger::on_artifact_created()
                .with_filter(ArtifactFlowFilter::new().with_artifact_types(vec![ArtifactType::Prd])),
        )
        .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("prd-library"))),
    );

    let prd = create_test_artifact(ArtifactType::Prd, None);
    let code = create_test_artifact(ArtifactType::CodeChange, None);

    let prd_evals = engine.on_artifact_created(&prd);
    assert_eq!(prd_evals.len(), 1);

    let code_evals = engine.on_artifact_created(&code);
    assert_eq!(code_evals.len(), 0);
}

#[test]
fn artifact_flow_engine_evaluate_triggers_inactive_flow_ignored() {
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Inactive", ArtifactFlowTrigger::on_artifact_created())
            .set_active(false),
    );

    let artifact = create_test_artifact(ArtifactType::Prd, None);
    let evals = engine.on_artifact_created(&artifact);
    assert_eq!(evals.len(), 0);
}

#[test]
fn artifact_flow_engine_evaluate_triggers_multiple_matches() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Flow A", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("a"))),
    );
    engine.register_flow(
        ArtifactFlow::new("Flow B", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("b"))),
    );

    let artifact = create_test_artifact(ArtifactType::Prd, None);
    let evals = engine.on_artifact_created(&artifact);
    assert_eq!(evals.len(), 2);
}

#[test]
fn artifact_flow_engine_on_task_completed() {
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Task Flow", ArtifactFlowTrigger::on_task_completed())
            .with_step(ArtifactFlowStep::spawn_process("archive", "system")),
    );

    let artifact = create_test_artifact(ArtifactType::CodeChange, None);
    let evals = engine.on_task_completed("task-1", Some(&artifact));
    assert_eq!(evals.len(), 1);
}

#[test]
fn artifact_flow_engine_on_process_completed() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Process Flow", ArtifactFlowTrigger::on_process_completed())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("archive"))),
    );

    let evals = engine.on_process_completed("process-1", None);
    assert_eq!(evals.len(), 1);
}

// ===== Research to Dev Flow Tests =====

#[test]
fn create_research_to_dev_flow_has_correct_structure() {
    let flow = create_research_to_dev_flow();
    assert_eq!(flow.name, "Research to Development");
    assert_eq!(flow.trigger.event, ArtifactFlowEvent::ArtifactCreated);
    assert!(flow.trigger.filter.is_some());

    let filter = flow.trigger.filter.as_ref().unwrap();
    assert_eq!(filter.artifact_types.as_ref().unwrap().len(), 1);
    assert_eq!(
        filter.artifact_types.as_ref().unwrap()[0],
        ArtifactType::Recommendations
    );
    assert_eq!(filter.source_bucket.as_ref().unwrap().as_str(), "research-outputs");

    assert_eq!(flow.steps.len(), 2);
    assert!(flow.steps[0].is_copy());
    assert!(flow.steps[1].is_spawn_process());
}

#[test]
fn research_to_dev_flow_triggers_correctly() {
    let flow = create_research_to_dev_flow();
    let engine = {
        let mut e = ArtifactFlowEngine::new();
        e.register_flow(flow);
        e
    };

    // Should match: recommendations in research-outputs
    let good = create_test_artifact(ArtifactType::Recommendations, Some("research-outputs"));
    let evals = engine.on_artifact_created(&good);
    assert_eq!(evals.len(), 1);

    // Should not match: wrong type
    let wrong_type = create_test_artifact(ArtifactType::Findings, Some("research-outputs"));
    let evals = engine.on_artifact_created(&wrong_type);
    assert_eq!(evals.len(), 0);

    // Should not match: wrong bucket
    let wrong_bucket = create_test_artifact(ArtifactType::Recommendations, Some("other"));
    let evals = engine.on_artifact_created(&wrong_bucket);
    assert_eq!(evals.len(), 0);
}

// ===== ArtifactUpdated Event Tests =====

#[test]
fn artifact_flow_event_includes_artifact_updated() {
    let all = ArtifactFlowEvent::all();
    assert!(all.contains(&ArtifactFlowEvent::ArtifactUpdated));
}

#[test]
fn artifact_flow_event_artifact_updated_serializes() {
    assert_eq!(
        serde_json::to_string(&ArtifactFlowEvent::ArtifactUpdated).unwrap(),
        "\"artifact_updated\""
    );
}

#[test]
fn artifact_flow_event_artifact_updated_parses() {
    let event: ArtifactFlowEvent = "artifact_updated".parse().unwrap();
    assert_eq!(event, ArtifactFlowEvent::ArtifactUpdated);
}

#[test]
fn artifact_flow_trigger_on_artifact_updated() {
    let trigger = ArtifactFlowTrigger::on_artifact_updated();
    assert_eq!(trigger.event, ArtifactFlowEvent::ArtifactUpdated);
    assert!(trigger.filter.is_none());
}

#[test]
fn artifact_flow_context_artifact_updated() {
    let artifact = create_test_artifact(ArtifactType::Specification, None);
    let context = ArtifactFlowContext::artifact_updated(artifact.clone());
    assert_eq!(context.event, ArtifactFlowEvent::ArtifactUpdated);
    assert!(context.artifact.is_some());
    assert!(context.task_id.is_none());
    assert!(context.process_id.is_none());
}

#[test]
fn artifact_flow_engine_on_artifact_updated() {
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Update Flow", ArtifactFlowTrigger::on_artifact_updated())
            .with_step(ArtifactFlowStep::emit_event("artifact:updated")),
    );

    let artifact = create_test_artifact(ArtifactType::Specification, None);
    let evals = engine.on_artifact_updated(&artifact);
    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].flow_name, "Update Flow");
}

#[test]
fn artifact_flow_engine_artifact_updated_does_not_trigger_created_flows() {
    use crate::domain::entities::artifact::ArtifactBucketId;
    let mut engine = ArtifactFlowEngine::new();
    engine.register_flow(
        ArtifactFlow::new("Create Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("target"))),
    );

    let artifact = create_test_artifact(ArtifactType::Specification, None);
    let evals = engine.on_artifact_updated(&artifact);
    assert_eq!(evals.len(), 0);
}

// ===== New Step Types Tests =====

#[test]
fn artifact_flow_step_emit_event_creates_correctly() {
    let step = ArtifactFlowStep::emit_event("plan:proposals_may_need_update");
    assert!(step.is_emit_event());
    assert_eq!(step.step_type(), "emit_event");

    if let ArtifactFlowStep::EmitEvent { event_name } = step {
        assert_eq!(event_name, "plan:proposals_may_need_update");
    } else {
        panic!("Expected EmitEvent step");
    }
}

#[test]
fn artifact_flow_step_emit_event_serializes() {
    let step = ArtifactFlowStep::emit_event("test:event");
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"emit_event\""));
    assert!(json.contains("\"test:event\""));
}

#[test]
fn artifact_flow_step_emit_event_deserializes() {
    let json = r#"{"type":"emit_event","event_name":"my:event"}"#;
    let step: ArtifactFlowStep = serde_json::from_str(json).unwrap();
    assert!(step.is_emit_event());
}

#[test]
fn artifact_flow_step_find_linked_proposals_creates_correctly() {
    let step = ArtifactFlowStep::find_linked_proposals();
    assert!(step.is_find_linked_proposals());
    assert_eq!(step.step_type(), "find_linked_proposals");
}

#[test]
fn artifact_flow_step_find_linked_proposals_serializes() {
    let step = ArtifactFlowStep::find_linked_proposals();
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"find_linked_proposals\""));
}

#[test]
fn artifact_flow_step_find_linked_proposals_deserializes() {
    let json = r#"{"type":"find_linked_proposals"}"#;
    let step: ArtifactFlowStep = serde_json::from_str(json).unwrap();
    assert!(step.is_find_linked_proposals());
}

// ===== Plan Updated Sync Flow Tests =====

#[test]
fn create_plan_updated_sync_flow_has_correct_structure() {
    let flow = create_plan_updated_sync_flow();
    assert_eq!(flow.name, "Plan Updated Sync");
    assert_eq!(flow.trigger.event, ArtifactFlowEvent::ArtifactUpdated);
    assert!(flow.trigger.filter.is_some());

    let filter = flow.trigger.filter.as_ref().unwrap();
    assert_eq!(filter.artifact_types.as_ref().unwrap().len(), 1);
    assert_eq!(
        filter.artifact_types.as_ref().unwrap()[0],
        ArtifactType::Specification
    );
    assert_eq!(filter.source_bucket.as_ref().unwrap().as_str(), "prd-library");

    assert_eq!(flow.steps.len(), 2);
    assert!(flow.steps[0].is_find_linked_proposals());
    assert!(flow.steps[1].is_emit_event());
}

#[test]
fn plan_updated_sync_flow_triggers_on_specification_update() {
    let flow = create_plan_updated_sync_flow();
    let engine = {
        let mut e = ArtifactFlowEngine::new();
        e.register_flow(flow);
        e
    };

    // Should match: specification in prd-library being updated
    let good = create_test_artifact(ArtifactType::Specification, Some("prd-library"));
    let evals = engine.on_artifact_updated(&good);
    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].flow_name, "Plan Updated Sync");
    assert_eq!(evals[0].steps.len(), 2);
}

#[test]
fn plan_updated_sync_flow_does_not_trigger_on_created() {
    let flow = create_plan_updated_sync_flow();
    let engine = {
        let mut e = ArtifactFlowEngine::new();
        e.register_flow(flow);
        e
    };

    // Should not match: artifact created (not updated)
    let artifact = create_test_artifact(ArtifactType::Specification, Some("prd-library"));
    let evals = engine.on_artifact_created(&artifact);
    assert_eq!(evals.len(), 0);
}

#[test]
fn plan_updated_sync_flow_does_not_trigger_wrong_type() {
    let flow = create_plan_updated_sync_flow();
    let engine = {
        let mut e = ArtifactFlowEngine::new();
        e.register_flow(flow);
        e
    };

    // Should not match: wrong artifact type
    let wrong_type = create_test_artifact(ArtifactType::Prd, Some("prd-library"));
    let evals = engine.on_artifact_updated(&wrong_type);
    assert_eq!(evals.len(), 0);
}

#[test]
fn plan_updated_sync_flow_does_not_trigger_wrong_bucket() {
    let flow = create_plan_updated_sync_flow();
    let engine = {
        let mut e = ArtifactFlowEngine::new();
        e.register_flow(flow);
        e
    };

    // Should not match: wrong bucket
    let wrong_bucket = create_test_artifact(ArtifactType::Specification, Some("other-bucket"));
    let evals = engine.on_artifact_updated(&wrong_bucket);
    assert_eq!(evals.len(), 0);
}
