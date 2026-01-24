// Artifact flow entities for the extensibility system
// Artifact flows automate artifact routing between processes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::artifact::{Artifact, ArtifactBucketId, ArtifactType};

/// A unique identifier for an ArtifactFlow
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactFlowId(pub String);

impl ArtifactFlowId {
    /// Creates a new ArtifactFlowId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ArtifactFlowId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ArtifactFlowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactFlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The event that triggers an artifact flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFlowEvent {
    /// Triggered when an artifact is created
    ArtifactCreated,
    /// Triggered when a task is completed
    TaskCompleted,
    /// Triggered when a process is completed
    ProcessCompleted,
}

impl ArtifactFlowEvent {
    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactFlowEvent::ArtifactCreated => "artifact_created",
            ArtifactFlowEvent::TaskCompleted => "task_completed",
            ArtifactFlowEvent::ProcessCompleted => "process_completed",
        }
    }

    /// Returns all flow events
    pub fn all() -> &'static [ArtifactFlowEvent] {
        &[
            ArtifactFlowEvent::ArtifactCreated,
            ArtifactFlowEvent::TaskCompleted,
            ArtifactFlowEvent::ProcessCompleted,
        ]
    }
}

impl fmt::Display for ArtifactFlowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing ArtifactFlowEvent from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseArtifactFlowEventError {
    pub value: String,
}

impl fmt::Display for ParseArtifactFlowEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown artifact flow event: '{}'", self.value)
    }
}

impl std::error::Error for ParseArtifactFlowEventError {}

impl FromStr for ArtifactFlowEvent {
    type Err = ParseArtifactFlowEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "artifact_created" => Ok(ArtifactFlowEvent::ArtifactCreated),
            "task_completed" => Ok(ArtifactFlowEvent::TaskCompleted),
            "process_completed" => Ok(ArtifactFlowEvent::ProcessCompleted),
            _ => Err(ParseArtifactFlowEventError {
                value: s.to_string(),
            }),
        }
    }
}

/// Filter criteria for artifact flow triggers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArtifactFlowFilter {
    /// Filter by artifact types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_types: Option<Vec<ArtifactType>>,
    /// Filter by source bucket
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_bucket: Option<ArtifactBucketId>,
}

impl ArtifactFlowFilter {
    /// Creates a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the artifact types filter
    pub fn with_artifact_types(mut self, types: Vec<ArtifactType>) -> Self {
        self.artifact_types = Some(types);
        self
    }

    /// Sets the source bucket filter
    pub fn with_source_bucket(mut self, bucket_id: ArtifactBucketId) -> Self {
        self.source_bucket = Some(bucket_id);
        self
    }

    /// Checks if an artifact matches this filter
    pub fn matches(&self, artifact: &Artifact) -> bool {
        // Check artifact type filter
        if let Some(ref types) = self.artifact_types {
            if !types.contains(&artifact.artifact_type) {
                return false;
            }
        }

        // Check source bucket filter
        if let Some(ref bucket_id) = self.source_bucket {
            if artifact.bucket_id.as_ref() != Some(bucket_id) {
                return false;
            }
        }

        true
    }

    /// Returns true if the filter has no criteria set
    pub fn is_empty(&self) -> bool {
        self.artifact_types.is_none() && self.source_bucket.is_none()
    }
}

/// The trigger configuration for an artifact flow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactFlowTrigger {
    /// The event that triggers this flow
    pub event: ArtifactFlowEvent,
    /// Optional filter to narrow the trigger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ArtifactFlowFilter>,
}

impl ArtifactFlowTrigger {
    /// Creates a new trigger for an event
    pub fn on_event(event: ArtifactFlowEvent) -> Self {
        Self {
            event,
            filter: None,
        }
    }

    /// Creates a trigger for artifact_created event
    pub fn on_artifact_created() -> Self {
        Self::on_event(ArtifactFlowEvent::ArtifactCreated)
    }

    /// Creates a trigger for task_completed event
    pub fn on_task_completed() -> Self {
        Self::on_event(ArtifactFlowEvent::TaskCompleted)
    }

    /// Creates a trigger for process_completed event
    pub fn on_process_completed() -> Self {
        Self::on_event(ArtifactFlowEvent::ProcessCompleted)
    }

    /// Sets the filter for this trigger
    pub fn with_filter(mut self, filter: ArtifactFlowFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Checks if an artifact matches this trigger's filter
    pub fn matches_artifact(&self, artifact: &Artifact) -> bool {
        match &self.filter {
            Some(filter) => filter.matches(artifact),
            None => true,
        }
    }
}

/// A step in an artifact flow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactFlowStep {
    /// Copy the artifact to another bucket
    Copy {
        /// The target bucket ID
        to_bucket: ArtifactBucketId,
    },
    /// Spawn a new process
    SpawnProcess {
        /// The type of process to spawn (e.g., "task_decomposition", "research")
        process_type: String,
        /// The agent profile to use for the process
        agent_profile: String,
    },
}

impl ArtifactFlowStep {
    /// Creates a copy step
    pub fn copy(to_bucket: ArtifactBucketId) -> Self {
        ArtifactFlowStep::Copy { to_bucket }
    }

    /// Creates a spawn_process step
    pub fn spawn_process(process_type: impl Into<String>, agent_profile: impl Into<String>) -> Self {
        ArtifactFlowStep::SpawnProcess {
            process_type: process_type.into(),
            agent_profile: agent_profile.into(),
        }
    }

    /// Returns true if this is a copy step
    pub fn is_copy(&self) -> bool {
        matches!(self, ArtifactFlowStep::Copy { .. })
    }

    /// Returns true if this is a spawn_process step
    pub fn is_spawn_process(&self) -> bool {
        matches!(self, ArtifactFlowStep::SpawnProcess { .. })
    }

    /// Returns the step type as a string
    pub fn step_type(&self) -> &'static str {
        match self {
            ArtifactFlowStep::Copy { .. } => "copy",
            ArtifactFlowStep::SpawnProcess { .. } => "spawn_process",
        }
    }
}

/// An artifact flow - automates artifact routing between processes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactFlow {
    /// Unique identifier
    pub id: ArtifactFlowId,
    /// Display name
    pub name: String,
    /// The trigger configuration
    pub trigger: ArtifactFlowTrigger,
    /// The steps to execute when triggered
    pub steps: Vec<ArtifactFlowStep>,
    /// Whether this flow is active
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    /// When the flow was created
    pub created_at: DateTime<Utc>,
}

fn default_is_active() -> bool {
    true
}

impl ArtifactFlow {
    /// Creates a new artifact flow
    pub fn new(name: impl Into<String>, trigger: ArtifactFlowTrigger) -> Self {
        Self {
            id: ArtifactFlowId::new(),
            name: name.into(),
            trigger,
            steps: vec![],
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Adds a step to the flow
    pub fn with_step(mut self, step: ArtifactFlowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Adds multiple steps to the flow
    pub fn with_steps(mut self, steps: impl IntoIterator<Item = ArtifactFlowStep>) -> Self {
        self.steps.extend(steps);
        self
    }

    /// Sets whether the flow is active
    pub fn set_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Checks if this flow should trigger for a given event and artifact
    pub fn should_trigger(&self, event: ArtifactFlowEvent, artifact: &Artifact) -> bool {
        if !self.is_active {
            return false;
        }
        if self.trigger.event != event {
            return false;
        }
        self.trigger.matches_artifact(artifact)
    }
}

/// Context for evaluating artifact flow triggers
#[derive(Debug, Clone)]
pub struct ArtifactFlowContext {
    /// The event that occurred
    pub event: ArtifactFlowEvent,
    /// The artifact involved (if any)
    pub artifact: Option<Artifact>,
    /// The task ID involved (if any)
    pub task_id: Option<String>,
    /// The process ID involved (if any)
    pub process_id: Option<String>,
}

impl ArtifactFlowContext {
    /// Creates a context for artifact_created event
    pub fn artifact_created(artifact: Artifact) -> Self {
        Self {
            event: ArtifactFlowEvent::ArtifactCreated,
            artifact: Some(artifact),
            task_id: None,
            process_id: None,
        }
    }

    /// Creates a context for task_completed event
    pub fn task_completed(task_id: impl Into<String>, artifact: Option<Artifact>) -> Self {
        Self {
            event: ArtifactFlowEvent::TaskCompleted,
            artifact,
            task_id: Some(task_id.into()),
            process_id: None,
        }
    }

    /// Creates a context for process_completed event
    pub fn process_completed(process_id: impl Into<String>, artifact: Option<Artifact>) -> Self {
        Self {
            event: ArtifactFlowEvent::ProcessCompleted,
            artifact,
            task_id: None,
            process_id: Some(process_id.into()),
        }
    }
}

/// Result of evaluating artifact flow triggers
#[derive(Debug, Clone)]
pub struct ArtifactFlowEvaluation {
    /// The flow ID that matched
    pub flow_id: ArtifactFlowId,
    /// The flow name
    pub flow_name: String,
    /// The steps to execute
    pub steps: Vec<ArtifactFlowStep>,
}

/// Engine for evaluating and executing artifact flows
#[derive(Debug, Default)]
pub struct ArtifactFlowEngine {
    /// Registered flows
    flows: Vec<ArtifactFlow>,
}

impl ArtifactFlowEngine {
    /// Creates a new empty engine
    pub fn new() -> Self {
        Self { flows: vec![] }
    }

    /// Registers a flow with the engine
    pub fn register_flow(&mut self, flow: ArtifactFlow) {
        self.flows.push(flow);
    }

    /// Registers multiple flows with the engine
    pub fn register_flows(&mut self, flows: impl IntoIterator<Item = ArtifactFlow>) {
        self.flows.extend(flows);
    }

    /// Removes a flow by ID
    pub fn unregister_flow(&mut self, flow_id: &ArtifactFlowId) -> Option<ArtifactFlow> {
        if let Some(pos) = self.flows.iter().position(|f| &f.id == flow_id) {
            Some(self.flows.remove(pos))
        } else {
            None
        }
    }

    /// Returns all registered flows
    pub fn flows(&self) -> &[ArtifactFlow] {
        &self.flows
    }

    /// Returns the number of registered flows
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Evaluates triggers for a given context and returns matching flows
    pub fn evaluate_triggers(&self, context: &ArtifactFlowContext) -> Vec<ArtifactFlowEvaluation> {
        let mut evaluations = vec![];

        for flow in &self.flows {
            if !flow.is_active {
                continue;
            }

            if flow.trigger.event != context.event {
                continue;
            }

            // Check filter if artifact is present
            let matches = match (&context.artifact, &flow.trigger.filter) {
                (Some(artifact), Some(filter)) => filter.matches(artifact),
                (Some(_artifact), None) => true,
                (None, Some(_filter)) => false, // Can't match filter without artifact
                (None, None) => true,           // No filter, no artifact, match
            };

            if matches {
                evaluations.push(ArtifactFlowEvaluation {
                    flow_id: flow.id.clone(),
                    flow_name: flow.name.clone(),
                    steps: flow.steps.clone(),
                });
            }
        }

        evaluations
    }

    /// Convenience method to evaluate triggers for artifact_created event
    pub fn on_artifact_created(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::artifact_created(artifact.clone());
        self.evaluate_triggers(&context)
    }

    /// Convenience method to evaluate triggers for task_completed event
    pub fn on_task_completed(
        &self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::task_completed(task_id, artifact.cloned());
        self.evaluate_triggers(&context)
    }

    /// Convenience method to evaluate triggers for process_completed event
    pub fn on_process_completed(
        &self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        let context = ArtifactFlowContext::process_completed(process_id, artifact.cloned());
        self.evaluate_triggers(&context)
    }
}

/// Creates the example "research-to-dev" flow from the PRD
pub fn create_research_to_dev_flow() -> ArtifactFlow {
    ArtifactFlow::new(
        "Research to Development",
        ArtifactFlowTrigger::on_artifact_created().with_filter(
            ArtifactFlowFilter::new()
                .with_artifact_types(vec![ArtifactType::Recommendations])
                .with_source_bucket(ArtifactBucketId::from_string("research-outputs")),
        ),
    )
    .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
        "prd-library",
    )))
    .with_step(ArtifactFlowStep::spawn_process(
        "task_decomposition",
        "orchestrator",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::artifact::{ArtifactContent, ArtifactMetadata};

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
    fn artifact_flow_event_all_returns_3_events() {
        let all = ArtifactFlowEvent::all();
        assert_eq!(all.len(), 3);
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
        use crate::domain::entities::artifact::ArtifactId;
        Artifact {
            id: ArtifactId::from_string("test-artifact"),
            artifact_type,
            name: "Test Artifact".to_string(),
            content: ArtifactContent::inline("Test content"),
            metadata: ArtifactMetadata::new("user"),
            derived_from: vec![],
            bucket_id: bucket_id.map(|s| ArtifactBucketId::from_string(s)),
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
        let flow = ArtifactFlow::new("Test", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string("target")));
        assert_eq!(flow.steps.len(), 1);
    }

    #[test]
    fn artifact_flow_with_steps() {
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
}
