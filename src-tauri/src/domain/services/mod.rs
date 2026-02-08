// Domain services - business logic that doesn't fit in entities
//
// Services coordinate repositories and entities to implement
// use cases and business rules.

pub mod artifact_flow_service;
pub mod artifact_service;
pub mod message_queue;
pub mod methodology_service;
pub mod research_service;
pub mod running_agent_registry;
pub mod workflow_service;

pub use artifact_flow_service::{ArtifactFlowService, FlowExecutionResult, StepExecutionResult};
pub use artifact_service::ArtifactService;
// Unified message queue - keyed by (context_type, context_id)
pub use message_queue::{MessageQueue, QueuedMessage, QueueKey};
pub use methodology_service::{MethodologyActivationResult, MethodologyService};
pub use research_service::ResearchService;
// Running agent registry for tracking and stopping agents
pub use running_agent_registry::{
    kill_process, MemoryRunningAgentRegistry, RunningAgentInfo, RunningAgentKey,
    RunningAgentRegistry,
};
pub use workflow_service::{
    AppliedColumn, AppliedWorkflow, ColumnMappingError, ValidationResult, WorkflowService,
};
