// Domain services - business logic that doesn't fit in entities
//
// Services coordinate repositories and entities to implement
// use cases and business rules.

pub mod api_key_service;
pub mod project_validation;
pub mod github_service;
pub mod artifact_flow_service;
pub mod artifact_service;
pub mod key_crypto;
pub mod bucket_classifier;
pub mod gap_fingerprint;
pub mod index_rewriter;
pub mod message_queue;
pub mod methodology_service;
pub mod research_service;
pub mod rule_ingestion_service;
pub mod rule_parser;
pub mod running_agent_registry;
pub mod verification_events;
pub mod verification_gate;
pub mod workflow_service;
pub mod worktree_guard;

pub use artifact_flow_service::{ArtifactFlowService, FlowExecutionResult, StepExecutionResult};
pub use github_service::{GithubServiceTrait, PrStatus};
pub use artifact_service::ArtifactService;
pub use bucket_classifier::BucketClassifier;
pub use gap_fingerprint::{gap_fingerprint, gap_score, jaccard_similarity};
pub use verification_events::{
    build_verification_payload, build_verification_started_metadata, emit_verification_started,
    emit_verification_status_changed,
};
pub use verification_gate::{
    check_proposal_verification_gate, check_verification_gate, ProposalOperation,
};
pub use index_rewriter::{IndexRewriter, RewriteResult};
// Unified message queue - keyed by (context_type, context_id)
pub use message_queue::{MessageQueue, QueueKey, QueuedMessage};
pub use methodology_service::{MethodologyActivationResult, MethodologyService};
pub use research_service::ResearchService;
pub use rule_ingestion_service::{IngestionResult, RuleIngestionService};
pub use rule_parser::{MarkdownChunk, ParsedRuleFile, RuleFrontmatter, RuleParser};
// Running agent registry for tracking and stopping agents
pub use running_agent_registry::{
    is_process_alive, kill_process, kill_process_immediate, kill_worktree_processes,
    kill_worktree_processes_async, MemoryRunningAgentRegistry, RunningAgentInfo, RunningAgentKey,
    RunningAgentRegistry,
};
pub use workflow_service::{
    AppliedColumn, AppliedWorkflow, ColumnMappingError, ValidationResult, WorkflowService,
};
pub use worktree_guard::{acquire_worktree_permit, is_worktree_in_use};
pub use project_validation::validate_project_path;
