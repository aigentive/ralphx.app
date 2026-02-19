// Shared test helpers for transition_handler tests
//
// Consolidated from side_effects.rs and tests.rs test infrastructure.
// All test files should `use super::helpers::*;` to access these.

pub use crate::application::{ChatService, MockChatService};
use crate::domain::entities::types::IdeationSessionId;
use crate::domain::entities::{
    ArtifactId, IdeationSession, IdeationSessionStatus, InternalStatus, PlanBranch,
    PlanBranchStatus, Project, ProjectId, Task, TaskCategory, TaskId,
};
pub use crate::domain::state_machine::context::{TaskContext, TaskServices};
pub use crate::domain::state_machine::mocks::{
    MockAgentSpawner, MockDependencyManager, MockEventEmitter, MockNotifier, MockReviewStarter,
};
pub use crate::domain::state_machine::services::{
    AgentSpawner, DependencyManager, EventEmitter, Notifier, ReviewStarter,
};
pub use std::sync::Arc;

// ==================
// Entity factories (from side_effects.rs tests)
// ==================

pub fn make_project(base_branch: Option<&str>) -> Project {
    let mut p = Project::new("test-project".into(), "/tmp/test".into());
    p.base_branch = base_branch.map(|s| s.to_string());
    p
}

pub fn make_task(plan_artifact_id: Option<&str>, task_branch: Option<&str>) -> Task {
    make_task_with_session(plan_artifact_id, task_branch, None)
}

pub fn make_task_with_session(
    plan_artifact_id: Option<&str>,
    task_branch: Option<&str>,
    ideation_session_id: Option<&str>,
) -> Task {
    let mut t = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        "Test task".into(),
    );
    t.plan_artifact_id = plan_artifact_id.map(|s| ArtifactId::from_string(s));
    t.task_branch = task_branch.map(|s| s.to_string());
    t.ideation_session_id = ideation_session_id.map(|s| IdeationSessionId::from_string(s));
    t
}

pub fn make_plan_branch(
    plan_artifact_id: &str,
    branch_name: &str,
    status: PlanBranchStatus,
    merge_task_id: Option<&str>,
) -> PlanBranch {
    let mut pb = PlanBranch::new(
        ArtifactId::from_string(plan_artifact_id),
        IdeationSessionId::from_string("sess-1"),
        ProjectId::from_string("proj-1".to_string()),
        branch_name.to_string(),
        "main".to_string(),
    );
    pb.status = status;
    pb.merge_task_id = merge_task_id.map(|s| TaskId::from_string(s.to_string()));
    pb
}

pub fn make_task_with_status(task_id: &str, status: InternalStatus) -> Task {
    let mut t = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        "Test task".into(),
    );
    t.id = TaskId::from_string(task_id.to_string());
    t.internal_status = status;
    t
}

pub fn make_task_with_category(category: TaskCategory) -> Task {
    let mut t = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        format!("Task with category {}", category),
    );
    t.category = category;
    t
}

pub fn make_session_no_title(session_id: &str) -> IdeationSession {
    let id = IdeationSessionId::from_string(session_id.to_string());
    IdeationSession {
        id,
        project_id: ProjectId::from_string("proj-1".to_string()),
        title: None,
        status: IdeationSessionStatus::default(),
        plan_artifact_id: None,
        seed_task_id: None,
        parent_session_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode: None,
        team_config_json: None,
        title_source: None,
    }
}

pub fn make_session_with_title_for_test(session_id: &str, title: &str) -> IdeationSession {
    let id = IdeationSessionId::from_string(session_id.to_string());
    IdeationSession {
        id,
        project_id: ProjectId::from_string("proj-1".to_string()),
        title: Some(title.to_string()),
        status: IdeationSessionStatus::default(),
        plan_artifact_id: None,
        seed_task_id: None,
        parent_session_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode: None,
        team_config_json: None,
        title_source: None,
    }
}

pub fn make_plan_task(session_id_str: &str, title: &str) -> Task {
    let mut t = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        title.to_string(),
    );
    t.ideation_session_id = Some(IdeationSessionId::from_string(session_id_str.to_string()));
    t
}

// ==================
// Service factories (from tests.rs)
// ==================

pub fn create_test_services() -> (
    Arc<MockAgentSpawner>,
    Arc<MockEventEmitter>,
    Arc<MockNotifier>,
    Arc<MockDependencyManager>,
    Arc<MockReviewStarter>,
    TaskServices,
) {
    let spawner = Arc::new(MockAgentSpawner::new());
    let emitter = Arc::new(MockEventEmitter::new());
    let notifier = Arc::new(MockNotifier::new());
    let dep_manager = Arc::new(MockDependencyManager::new());
    let review_starter = Arc::new(MockReviewStarter::new());
    let chat_service = Arc::new(MockChatService::new());

    let services = TaskServices::new(
        Arc::clone(&spawner) as Arc<dyn AgentSpawner>,
        Arc::clone(&emitter) as Arc<dyn EventEmitter>,
        Arc::clone(&notifier) as Arc<dyn Notifier>,
        Arc::clone(&dep_manager) as Arc<dyn DependencyManager>,
        Arc::clone(&review_starter) as Arc<dyn ReviewStarter>,
        Arc::clone(&chat_service) as Arc<dyn ChatService>,
    );

    (
        spawner,
        emitter,
        notifier,
        dep_manager,
        review_starter,
        services,
    )
}

pub fn create_context_with_services(
    task_id: &str,
    project_id: &str,
    services: TaskServices,
) -> TaskContext {
    TaskContext::new(task_id, project_id, services)
}
