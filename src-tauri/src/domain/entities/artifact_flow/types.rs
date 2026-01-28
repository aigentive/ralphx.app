// Type definitions for artifact flow entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::super::artifact::{Artifact, ArtifactBucketId, ArtifactType};

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
    /// Triggered when an artifact is updated (new version created)
    ArtifactUpdated,
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
            ArtifactFlowEvent::ArtifactUpdated => "artifact_updated",
            ArtifactFlowEvent::TaskCompleted => "task_completed",
            ArtifactFlowEvent::ProcessCompleted => "process_completed",
        }
    }

    /// Returns all flow events
    pub fn all() -> &'static [ArtifactFlowEvent] {
        &[
            ArtifactFlowEvent::ArtifactCreated,
            ArtifactFlowEvent::ArtifactUpdated,
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
            "artifact_updated" => Ok(ArtifactFlowEvent::ArtifactUpdated),
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

    /// Creates a trigger for artifact_updated event
    pub fn on_artifact_updated() -> Self {
        Self::on_event(ArtifactFlowEvent::ArtifactUpdated)
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
    /// Emit a Tauri event to the frontend
    EmitEvent {
        /// The event name (e.g., "plan:proposals_may_need_update")
        event_name: String,
    },
    /// Find proposals linked to a plan artifact (for proactive sync)
    FindLinkedProposals,
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

    /// Creates an emit_event step
    pub fn emit_event(event_name: impl Into<String>) -> Self {
        ArtifactFlowStep::EmitEvent {
            event_name: event_name.into(),
        }
    }

    /// Creates a find_linked_proposals step
    pub fn find_linked_proposals() -> Self {
        ArtifactFlowStep::FindLinkedProposals
    }

    /// Returns true if this is a copy step
    pub fn is_copy(&self) -> bool {
        matches!(self, ArtifactFlowStep::Copy { .. })
    }

    /// Returns true if this is a spawn_process step
    pub fn is_spawn_process(&self) -> bool {
        matches!(self, ArtifactFlowStep::SpawnProcess { .. })
    }

    /// Returns true if this is an emit_event step
    pub fn is_emit_event(&self) -> bool {
        matches!(self, ArtifactFlowStep::EmitEvent { .. })
    }

    /// Returns true if this is a find_linked_proposals step
    pub fn is_find_linked_proposals(&self) -> bool {
        matches!(self, ArtifactFlowStep::FindLinkedProposals)
    }

    /// Returns the step type as a string
    pub fn step_type(&self) -> &'static str {
        match self {
            ArtifactFlowStep::Copy { .. } => "copy",
            ArtifactFlowStep::SpawnProcess { .. } => "spawn_process",
            ArtifactFlowStep::EmitEvent { .. } => "emit_event",
            ArtifactFlowStep::FindLinkedProposals => "find_linked_proposals",
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

    /// Creates a context for artifact_updated event
    pub fn artifact_updated(artifact: Artifact) -> Self {
        Self {
            event: ArtifactFlowEvent::ArtifactUpdated,
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
