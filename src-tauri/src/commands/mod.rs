// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod agent_profile_commands;
pub mod artifact_commands;
pub mod execution_commands;
pub mod health;
pub mod ideation_commands;
pub mod project_commands;
pub mod qa_commands;
pub mod review_commands;
pub mod task_commands;
pub mod workflow_commands;

// Re-export commands for registration
pub use agent_profile_commands::{
    get_agent_profile, get_agent_profiles_by_role, get_builtin_agent_profiles,
    get_custom_agent_profiles, list_agent_profiles, seed_builtin_profiles,
};
pub use artifact_commands::{
    add_artifact_relation, create_artifact, create_bucket, delete_artifact, get_artifact,
    get_artifact_relations, get_artifacts, get_artifacts_by_bucket, get_artifacts_by_task,
    get_buckets, get_system_buckets, update_artifact, AddRelationInput, ArtifactRelationResponse,
    ArtifactResponse, BucketResponse, CreateArtifactInput, CreateBucketInput, UpdateArtifactInput,
};
pub use execution_commands::{
    get_execution_status, pause_execution, resume_execution, stop_execution, ExecutionState,
};
pub use health::health_check;
pub use ideation_commands::{
    add_proposal_dependency, analyze_dependencies, apply_proposals_to_kanban,
    archive_ideation_session, assess_all_priorities, assess_proposal_priority,
    count_session_messages, create_ideation_session, create_task_proposal,
    delete_chat_message, delete_ideation_session, delete_session_messages, delete_task_proposal,
    get_blocked_tasks, get_ideation_session, get_ideation_session_with_data,
    get_project_messages, get_proposal_dependencies, get_proposal_dependents,
    get_recent_session_messages, get_session_messages, get_task_blockers, get_task_messages,
    get_task_proposal, is_orchestrator_available, list_ideation_sessions, list_session_proposals,
    remove_proposal_dependency, reorder_proposals, send_chat_message, send_orchestrator_message,
    set_proposal_selection, toggle_proposal_selection, update_task_proposal,
    ApplyProposalsResultResponse, ChatMessageResponse, DependencyGraphResponse,
    IdeationSessionResponse, OrchestratorMessageResponse, PriorityAssessmentResponse,
    SessionWithDataResponse, TaskProposalResponse, ToolCallResultResponse,
};
pub use project_commands::{
    create_project, delete_project, get_project, list_projects, update_project,
};
pub use qa_commands::{
    get_qa_results, get_qa_settings, get_task_qa, retry_qa, skip_qa, update_qa_settings,
};
pub use review_commands::{
    approve_fix_task, approve_review, get_fix_task_attempts, get_pending_reviews,
    get_review_by_id, get_reviews_by_task_id, get_task_state_history, reject_fix_task,
    reject_review, request_changes,
};
pub use task_commands::{answer_user_question, create_task, delete_task, get_task, inject_task, list_tasks, update_task};
pub use workflow_commands::{
    create_workflow, delete_workflow, get_active_workflow_columns, get_builtin_workflows,
    get_workflow, get_workflows, set_default_workflow, update_workflow, CreateWorkflowInput,
    UpdateWorkflowInput, WorkflowColumnInput, WorkflowColumnResponse, WorkflowResponse,
};
