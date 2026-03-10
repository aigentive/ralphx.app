use super::*;
use crate::application::AppState;
use crate::commands::ideation_commands::ideation_commands_apply::should_create_feature_branch;
use crate::domain::entities::{
    ChatMessage, IdeationSession, IdeationSessionId, IdeationSessionStatus, Priority, ProjectId,
    ProposalCategory, TaskProposal, TaskProposalId,
};
use crate::domain::ideation::IdeationSettings;

fn setup_test_state() -> AppState {
    AppState::new_test()
}

#[tokio::test]
async fn test_create_ideation_session_without_title() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id.clone());
    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    assert_eq!(created.project_id, project_id);
    assert!(created.title.is_none());
    assert_eq!(created.status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_create_ideation_session_with_title() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    assert_eq!(created.project_id, project_id);
    assert_eq!(created.title, Some("Test Session".to_string()));
    assert_eq!(created.status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_get_ideation_session_returns_none_for_nonexistent() {
    let state = setup_test_state();
    let id = IdeationSessionId::new();

    let result = state
        .ideation_session_repo
        .get_by_id(&id)
        .await
        .expect("Failed to get ideation session by id in test");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_ideation_session_returns_existing() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id);
    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let result = state
        .ideation_session_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get ideation session by id in test");
    assert!(result.is_some());
    assert_eq!(result.expect("Expected to find session").id, created.id);
}

#[tokio::test]
async fn test_list_ideation_sessions_by_project() {
    let state = setup_test_state();
    let project_id = ProjectId::new();
    let other_project_id = ProjectId::new();

    // Create sessions for our project
    state
        .ideation_session_repo
        .create(IdeationSession::new(project_id.clone()))
        .await
        .expect("Failed to create ideation session in test");
    state
        .ideation_session_repo
        .create(IdeationSession::new(project_id.clone()))
        .await
        .expect("Failed to create ideation session in test");

    // Create session for different project
    state
        .ideation_session_repo
        .create(IdeationSession::new(other_project_id))
        .await
        .expect("Failed to create ideation session in test");

    let sessions = state
        .ideation_session_repo
        .get_by_project(&project_id)
        .await
        .expect("Failed to get sessions by project in test");
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn test_archive_ideation_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id);
    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    state
        .ideation_session_repo
        .update_status(&created.id, IdeationSessionStatus::Archived)
        .await
        .expect("Failed to update ideation session status in test");

    let retrieved = state
        .ideation_session_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get by id in test")
        .expect("Expected to find entity");
    assert_eq!(retrieved.status, IdeationSessionStatus::Archived);
}

#[tokio::test]
async fn test_delete_ideation_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    let session = IdeationSession::new(project_id);
    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    state
        .ideation_session_repo
        .delete(&created.id)
        .await
        .expect("Failed to delete ideation session in test");

    let result = state
        .ideation_session_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get by id in test");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_session_response_serialization() {
    let project_id = ProjectId::new();
    let session = IdeationSession::new_with_title(project_id, "Test Session");
    let response = IdeationSessionResponse::from(session);

    assert!(!response.id.is_empty());
    assert_eq!(response.title, Some("Test Session".to_string()));
    assert_eq!(response.status, "active");

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize response in test");
    assert!(json.contains("\"title\":\"Test Session\""));
}

#[tokio::test]
async fn test_get_session_with_data() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create proposal for session
    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "Test Proposal",
        crate::domain::entities::ProposalCategory::Feature,
        crate::domain::entities::Priority::High,
    );
    state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create task proposal in test");

    // Create message for session
    let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello");
    state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    // Get session with data
    let proposals = state
        .task_proposal_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get proposals by session in test");
    let messages = state
        .chat_message_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get messages by session in test");

    assert_eq!(proposals.len(), 1);
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_proposal_response_serialization() {
    let session_id = IdeationSessionId::new();
    let proposal = TaskProposal::new(
        session_id,
        "Test Proposal",
        crate::domain::entities::ProposalCategory::Feature,
        crate::domain::entities::Priority::High,
    );
    let response = TaskProposalResponse::from(proposal);

    assert!(!response.id.is_empty());
    assert_eq!(response.title, "Test Proposal");
    assert_eq!(response.category, "feature");
    assert_eq!(response.suggested_priority, "high");

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize response in test");
    assert!(json.contains("\"title\":\"Test Proposal\""));
}

#[tokio::test]
async fn test_message_response_serialization() {
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::user_in_session(session_id, "Hello world");
    let response = ChatMessageResponse::from(message);

    assert!(!response.id.is_empty());
    assert_eq!(response.content, "Hello world");
    assert_eq!(response.role, "user");

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize response in test");
    assert!(json.contains("\"content\":\"Hello world\""));
}

// ========================================================================
// Task Proposal Tests
// ========================================================================

#[tokio::test]
async fn test_create_task_proposal() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session first
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create proposal
    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "Test Proposal",
        ProposalCategory::Feature,
        Priority::High,
    );
    let created = state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create task proposal in test");

    assert_eq!(created.title, "Test Proposal");
    assert_eq!(created.category, ProposalCategory::Feature);
    assert_eq!(created.suggested_priority, Priority::High);
}

#[tokio::test]
async fn test_get_task_proposal_returns_none_for_nonexistent() {
    let state = setup_test_state();
    let id = TaskProposalId::new();

    let result = state
        .task_proposal_repo
        .get_by_id(&id)
        .await
        .expect("Failed to get task proposal by id in test");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_list_session_proposals() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create proposals
    for i in 0..3 {
        let proposal = TaskProposal::new(
            created_session.id.clone(),
            format!("Proposal {}", i),
            ProposalCategory::Feature,
            Priority::Medium,
        );
        state
            .task_proposal_repo
            .create(proposal)
            .await
            .expect("Failed to create task proposal in test");
    }

    let proposals = state
        .task_proposal_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get proposals by session in test");
    assert_eq!(proposals.len(), 3);
}

#[tokio::test]
async fn test_update_task_proposal() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposal
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "Original Title",
        ProposalCategory::Feature,
        Priority::Low,
    );
    let created = state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create task proposal in test");

    // Update proposal
    let mut updated = created.clone();
    updated.title = "Updated Title".to_string();
    updated.user_modified = true;

    state
        .task_proposal_repo
        .update(&updated)
        .await
        .expect("Failed to update task proposal in test");

    let retrieved = state
        .task_proposal_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get by id in test")
        .expect("Expected to find entity");
    assert_eq!(retrieved.title, "Updated Title");
    assert!(retrieved.user_modified);
}

#[tokio::test]
async fn test_delete_task_proposal() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposal
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "To Delete",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let created = state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create task proposal in test");

    // Delete proposal
    state
        .task_proposal_repo
        .delete(&created.id)
        .await
        .expect("Failed to delete task proposal in test");

    let result = state
        .task_proposal_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get by id in test");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_toggle_proposal_selection() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposal
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "Test Proposal",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let created = state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create task proposal in test");

    // Initial state should be selected (false)
    assert!(!created.selected);

    // Toggle to true
    state
        .task_proposal_repo
        .update_selection(&created.id, true)
        .await
        .expect("Failed to update selection in test");

    let retrieved = state
        .task_proposal_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get by id in test")
        .expect("Expected to find entity");
    assert!(retrieved.selected);
}

#[tokio::test]
async fn test_reorder_proposals() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create 3 proposals
    let mut ids = Vec::new();
    for i in 0..3 {
        let proposal = TaskProposal::new(
            created_session.id.clone(),
            format!("Proposal {}", i),
            ProposalCategory::Feature,
            Priority::Medium,
        );
        let created = state
            .task_proposal_repo
            .create(proposal)
            .await
            .expect("Failed to create task proposal in test");
        ids.push(created.id);
    }

    // Reverse the order
    let reversed_ids: Vec<TaskProposalId> = ids.into_iter().rev().collect();
    state
        .task_proposal_repo
        .reorder(&created_session.id, reversed_ids)
        .await
        .expect("Failed to reorder proposals in test");

    // Verify order changed
    let proposals = state
        .task_proposal_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get proposals by session in test");
    assert_eq!(proposals.len(), 3);
    // The first proposal should now be "Proposal 2"
    assert_eq!(proposals[0].title, "Proposal 2");
}

#[tokio::test]
async fn test_priority_assessment_response() {
    let session_id = IdeationSessionId::new();
    let proposal = TaskProposal::new(
        session_id,
        "Test Proposal",
        ProposalCategory::Feature,
        Priority::Critical,
    );

    let response = PriorityAssessmentResponse {
        proposal_id: proposal.id.as_str().to_string(),
        priority: proposal.suggested_priority.to_string(),
        score: proposal.priority_score,
        reason: "Test reason".to_string(),
    };

    assert_eq!(response.priority, "critical");
    assert_eq!(response.reason, "Test reason");

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize response in test");
    assert!(json.contains("\"priority\":\"critical\""));
}

// ========================================================================
// Dependency and Apply Tests
// ========================================================================

#[tokio::test]
async fn test_add_proposal_dependency() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposals
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal1 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 1",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let proposal2 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 2",
        ProposalCategory::Feature,
        Priority::Medium,
    );

    let p1 = state
        .task_proposal_repo
        .create(proposal1)
        .await
        .expect("Failed to create task proposal in test");
    let p2 = state
        .task_proposal_repo
        .create(proposal2)
        .await
        .expect("Failed to create task proposal in test");

    // Add dependency: p1 depends on p2
    state
        .proposal_dependency_repo
        .add_dependency(&p1.id, &p2.id, None, None)
        .await
        .expect("Failed to add dependency in test");

    // Verify dependency exists
    let deps = state
        .proposal_dependency_repo
        .get_dependencies(&p1.id)
        .await
        .expect("Failed to get dependencies in test");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], p2.id);
}

#[tokio::test]
async fn test_remove_proposal_dependency() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposals
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal1 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 1",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let proposal2 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 2",
        ProposalCategory::Feature,
        Priority::Medium,
    );

    let p1 = state
        .task_proposal_repo
        .create(proposal1)
        .await
        .expect("Failed to create task proposal in test");
    let p2 = state
        .task_proposal_repo
        .create(proposal2)
        .await
        .expect("Failed to create task proposal in test");

    // Add then remove dependency
    state
        .proposal_dependency_repo
        .add_dependency(&p1.id, &p2.id, None, None)
        .await
        .expect("Failed to add dependency in test");
    state
        .proposal_dependency_repo
        .remove_dependency(&p1.id, &p2.id)
        .await
        .expect("Failed to remove dependency in test");

    // Verify dependency was removed
    let deps = state
        .proposal_dependency_repo
        .get_dependencies(&p1.id)
        .await
        .expect("Failed to get dependencies in test");
    assert!(deps.is_empty());
}

#[tokio::test]
async fn test_get_proposal_dependents() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and proposals
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let proposal1 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 1",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let proposal2 = TaskProposal::new(
        created_session.id.clone(),
        "Proposal 2",
        ProposalCategory::Feature,
        Priority::Medium,
    );

    let p1 = state
        .task_proposal_repo
        .create(proposal1)
        .await
        .expect("Failed to create task proposal in test");
    let p2 = state
        .task_proposal_repo
        .create(proposal2)
        .await
        .expect("Failed to create task proposal in test");

    // p1 depends on p2, so p2 should have p1 as a dependent
    state
        .proposal_dependency_repo
        .add_dependency(&p1.id, &p2.id, None, None)
        .await
        .expect("Failed to add dependency in test");

    let dependents = state
        .proposal_dependency_repo
        .get_dependents(&p2.id)
        .await
        .expect("Failed to get dependents in test");
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0], p1.id);
}

#[tokio::test]
async fn test_analyze_dependencies_empty_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session with no proposals
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Get dependencies (should be empty graph)
    let proposals = state
        .task_proposal_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get proposals by session in test");
    let deps = state
        .proposal_dependency_repo
        .get_all_for_session(&created_session.id)
        .await
        .expect("Failed to get all dependencies for session in test");

    let graph = build_dependency_graph(&proposals, &deps);

    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
    assert!(!graph.has_cycles);
}

#[tokio::test]
async fn test_dependency_graph_response_serialization() {
    use crate::domain::entities::DependencyGraph;

    let graph = DependencyGraph::new();
    let response = DependencyGraphResponse::from(graph);

    assert!(response.nodes.is_empty());
    assert!(response.edges.is_empty());
    assert!(!response.has_cycles);

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).expect("Failed to serialize response in test");
    assert!(json.contains("\"has_cycles\":false"));
}

#[tokio::test]
async fn test_task_blockers() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create tasks
    let task1 = crate::domain::entities::Task::new(project_id.clone(), "Task 1".to_string());
    let task2 = crate::domain::entities::Task::new(project_id.clone(), "Task 2".to_string());

    let t1 = state
        .task_repo
        .create(task1)
        .await
        .expect("Failed to create task in test");
    let t2 = state
        .task_repo
        .create(task2)
        .await
        .expect("Failed to create task in test");

    // Add dependency: t1 depends on t2
    state
        .task_dependency_repo
        .add_dependency(&t1.id, &t2.id)
        .await
        .expect("Failed to add task dependency in test");

    // t2 should be a blocker for t1
    let blockers = state
        .task_dependency_repo
        .get_blockers(&t1.id)
        .await
        .expect("Failed to get blockers in test");
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0], t2.id);

    // t1 should be blocked by t2
    let blocked = state
        .task_dependency_repo
        .get_blocked_by(&t2.id)
        .await
        .expect("Failed to get blocked tasks in test");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0], t1.id);
}

// ========================================================================
// Chat Message Tests
// ========================================================================

#[tokio::test]
async fn test_send_chat_message_to_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session first
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Send a message
    let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello world");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    assert_eq!(created.content, "Hello world");
    assert_eq!(created.session_id, Some(created_session.id));
}

#[tokio::test]
async fn test_send_chat_message_to_project() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Send a message to project
    let message = ChatMessage::user_in_project(project_id.clone(), "Project message");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    assert_eq!(created.content, "Project message");
    assert_eq!(created.project_id, Some(project_id));
    assert!(created.session_id.is_none());
}

#[tokio::test]
async fn test_send_chat_message_about_task() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create a task
    let task = crate::domain::entities::Task::new(project_id, "Test Task".to_string());
    let created_task = state
        .task_repo
        .create(task)
        .await
        .expect("Failed to create task in test");

    // Send a message about the task
    let message = ChatMessage::user_about_task(created_task.id.clone(), "Task message");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    assert_eq!(created.content, "Task message");
    assert_eq!(created.task_id, Some(created_task.id));
}

#[tokio::test]
async fn test_get_session_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Send multiple messages
    for i in 1..=3 {
        let message =
            ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Get all messages
    let messages = state
        .chat_message_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get messages by session in test");
    assert_eq!(messages.len(), 3);
}

#[tokio::test]
async fn test_get_project_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Send messages to project
    for i in 1..=2 {
        let message =
            ChatMessage::user_in_project(project_id.clone(), format!("Project message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Get all project messages
    let messages = state
        .chat_message_repo
        .get_by_project(&project_id)
        .await
        .expect("Failed to get sessions by project in test");
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_get_task_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create a task
    let task = crate::domain::entities::Task::new(project_id, "Test Task".to_string());
    let created_task = state
        .task_repo
        .create(task)
        .await
        .expect("Failed to create task in test");

    // Send messages about the task
    for i in 1..=2 {
        let message =
            ChatMessage::user_about_task(created_task.id.clone(), format!("Task message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Get all task messages
    let messages = state
        .chat_message_repo
        .get_by_task(&created_task.id)
        .await
        .expect("Failed to get messages by task in test");
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_delete_chat_message() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session and message
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    let message = ChatMessage::user_in_session(created_session.id.clone(), "To delete");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    // Delete the message
    state
        .chat_message_repo
        .delete(&created.id)
        .await
        .expect("Failed to delete chat message in test");

    // Verify it's gone
    let result = state
        .chat_message_repo
        .get_by_id(&created.id)
        .await
        .expect("Failed to get chat message by id in test");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_session_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create multiple messages
    for i in 1..=3 {
        let message =
            ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Delete all session messages
    state
        .chat_message_repo
        .delete_by_session(&created_session.id)
        .await
        .expect("Failed to delete messages by session in test");

    // Verify they're gone
    let messages = state
        .chat_message_repo
        .get_by_session(&created_session.id)
        .await
        .expect("Failed to get messages by session in test");
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_count_session_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create messages
    for i in 1..=5 {
        let message =
            ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Count messages
    let count = state
        .chat_message_repo
        .count_by_session(&created_session.id)
        .await
        .expect("Failed to count messages by session in test");
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_get_recent_session_messages() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create 5 messages
    for i in 1..=5 {
        let message =
            ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
        state
            .chat_message_repo
            .create(message)
            .await
            .expect("Failed to create chat message in test");
    }

    // Get only 3 recent messages
    let messages = state
        .chat_message_repo
        .get_recent_by_session(&created_session.id, 3)
        .await
        .expect("Failed to get recent messages by session in test");
    assert_eq!(messages.len(), 3);
}

#[tokio::test]
async fn test_chat_message_response_includes_all_fields() {
    let session_id = IdeationSessionId::new();
    let mut message = ChatMessage::user_in_session(session_id.clone(), "Test message");
    message.metadata = Some(r#"{"key": "value"}"#.to_string());

    let response = ChatMessageResponse::from(message.clone());

    assert_eq!(response.content, "Test message");
    assert_eq!(response.role, "user");
    assert_eq!(response.session_id, Some(session_id.as_str().to_string()));
    assert!(response.project_id.is_none());
    assert!(response.task_id.is_none());
    assert_eq!(response.metadata, Some(r#"{"key": "value"}"#.to_string()));
}

#[tokio::test]
async fn test_orchestrator_message_in_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create orchestrator message
    let message = ChatMessage::orchestrator_in_session(created_session.id.clone(), "AI response");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    let response = ChatMessageResponse::from(created);
    assert_eq!(response.role, "orchestrator");
    assert_eq!(response.content, "AI response");
}

#[tokio::test]
async fn test_system_message_in_session() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create ideation session in test");

    // Create system message
    let message = ChatMessage::system_in_session(created_session.id.clone(), "Session started");
    let created = state
        .chat_message_repo
        .create(message)
        .await
        .expect("Failed to create chat message in test");

    let response = ChatMessageResponse::from(created);
    assert_eq!(response.role, "system");
    assert_eq!(response.content, "Session started");
}

// ========================================================================
// Ideation Settings Tests
// ========================================================================

#[tokio::test]
async fn test_get_ideation_settings_returns_default() {
    let state = setup_test_state();

    // Get settings (should return default)
    let settings = state
        .ideation_settings_repo
        .get_settings()
        .await
        .expect("Failed to get ideation settings in test");

    assert_eq!(
        settings.plan_mode,
        crate::domain::ideation::IdeationPlanMode::Optional
    );
    assert!(!settings.require_plan_approval);
    assert!(settings.suggest_plans_for_complex);
    assert!(settings.auto_link_proposals);
}

#[tokio::test]
async fn test_update_ideation_settings() {
    let state = setup_test_state();

    // Create custom settings
    let custom_settings = IdeationSettings {
        plan_mode: crate::domain::ideation::IdeationPlanMode::Required,
        require_plan_approval: true,
        suggest_plans_for_complex: false,
        auto_link_proposals: false,
        require_verification_for_accept: false,
        require_verification_for_proposals: false,
    };

    // Update settings
    let updated = state
        .ideation_settings_repo
        .update_settings(&custom_settings)
        .await
        .expect("Failed to update ideation settings in test");

    assert_eq!(
        updated.plan_mode,
        crate::domain::ideation::IdeationPlanMode::Required
    );
    assert!(updated.require_plan_approval);
    assert!(!updated.suggest_plans_for_complex);
    assert!(!updated.auto_link_proposals);
}

#[tokio::test]
async fn test_ideation_settings_persist_across_reads() {
    let state = setup_test_state();

    // Update settings
    let custom_settings = IdeationSettings {
        plan_mode: crate::domain::ideation::IdeationPlanMode::Parallel,
        require_plan_approval: false,
        suggest_plans_for_complex: true,
        auto_link_proposals: false,
        require_verification_for_accept: false,
        require_verification_for_proposals: false,
    };

    state
        .ideation_settings_repo
        .update_settings(&custom_settings)
        .await
        .expect("Failed to update ideation settings in test");

    // Read settings again
    let retrieved = state
        .ideation_settings_repo
        .get_settings()
        .await
        .expect("Failed to get ideation settings in test");

    assert_eq!(
        retrieved.plan_mode,
        crate::domain::ideation::IdeationPlanMode::Parallel
    );
    assert!(!retrieved.require_plan_approval);
    assert!(retrieved.suggest_plans_for_complex);
    assert!(!retrieved.auto_link_proposals);
}

// ========================================================================
// Cascade Delete Session Tests (Phase 103)
// ========================================================================

#[tokio::test]
async fn test_delete_session_cascades_to_tasks() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    // Create tasks linked to this session
    let mut task1 = crate::domain::entities::Task::new(project_id.clone(), "Task 1".to_string());
    task1.ideation_session_id = Some(created_session.id.clone());
    let mut task2 = crate::domain::entities::Task::new(project_id.clone(), "Task 2".to_string());
    task2.ideation_session_id = Some(created_session.id.clone());

    let t1 = state
        .task_repo
        .create(task1)
        .await
        .expect("Failed to create task");
    let t2 = state
        .task_repo
        .create(task2)
        .await
        .expect("Failed to create task");

    // Verify tasks exist and are linked to session
    let session_tasks = state
        .task_repo
        .get_by_ideation_session(&created_session.id)
        .await
        .expect("Failed to query tasks");
    assert_eq!(session_tasks.len(), 2);

    // Simulate cascade: delete tasks then session (mirrors command logic)
    for task in &session_tasks {
        state
            .task_repo
            .delete(&task.id)
            .await
            .expect("Failed to delete task");
    }
    state
        .ideation_session_repo
        .delete(&created_session.id)
        .await
        .expect("Failed to delete session");

    // Verify tasks are gone
    assert!(state.task_repo.get_by_id(&t1.id).await.unwrap().is_none());
    assert!(state.task_repo.get_by_id(&t2.id).await.unwrap().is_none());

    // Verify session is gone
    assert!(state
        .ideation_session_repo
        .get_by_id(&created_session.id)
        .await
        .unwrap()
        .is_none());

    // Verify get_by_ideation_session returns empty
    let remaining = state
        .task_repo
        .get_by_ideation_session(&created_session.id)
        .await
        .unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn test_delete_session_with_no_tasks() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session with no tasks
    let session = IdeationSession::new(project_id);
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    // Verify no tasks for this session
    let session_tasks = state
        .task_repo
        .get_by_ideation_session(&created_session.id)
        .await
        .unwrap();
    assert!(session_tasks.is_empty());

    // Delete session directly (no cascade needed)
    state
        .ideation_session_repo
        .delete(&created_session.id)
        .await
        .expect("Failed to delete session");

    // Verify session is gone
    assert!(state
        .ideation_session_repo
        .get_by_id(&created_session.id)
        .await
        .unwrap()
        .is_none());
}

// ========================================================================
// Feature Branch Decision Tests
// ========================================================================

#[test]
fn test_feature_branch_uses_project_default() {
    // When no override is set, use project default
    assert!(should_create_feature_branch(None, true));
    assert!(!should_create_feature_branch(None, false));
}

#[test]
fn test_feature_branch_respects_override() {
    // Override takes precedence over project default
    assert!(should_create_feature_branch(Some(true), false));
    assert!(!should_create_feature_branch(Some(false), true));
}

#[test]
fn test_single_proposal_now_creates_feature_branch_when_enabled() {
    // Regression test: single-proposal plans now respect the project setting.
    // Previously, single-proposal plans skipped feature branches unconditionally.
    assert!(should_create_feature_branch(None, true));
    assert!(should_create_feature_branch(Some(true), true));
}

#[tokio::test]
async fn test_single_proposal_now_creates_feature_branch() {
    // Integration test: single-proposal plan now respects feature branch setting.
    // When feature branches are enabled, a single proposal should create a feature branch.
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create session
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    // Create 1 proposal
    let proposal = TaskProposal::new(
        created_session.id.clone(),
        "Solo Proposal",
        ProposalCategory::Feature,
        Priority::Medium,
    );
    let p1 = state
        .task_proposal_repo
        .create(proposal)
        .await
        .expect("Failed to create proposal");

    // Create the corresponding task (simulating what apply_proposals_to_kanban does)
    let mut task = crate::domain::entities::Task::new(project_id.clone(), p1.title.clone());
    task.ideation_session_id = Some(created_session.id.clone());
    let _created_task = state
        .task_repo
        .create(task)
        .await
        .expect("Failed to create task");

    // Decision: should_create_feature_branch now returns true for project default enabled
    let should_create = should_create_feature_branch(None, true);
    assert!(
        should_create,
        "Feature branch should now be created for single-proposal plan when enabled"
    );

    // Note: The actual plan_branch record and merge task creation happens in
    // apply_proposals_to_kanban, not in this test. This test only verifies
    // the decision function returns the correct value.
}

#[tokio::test]
async fn test_delete_session_only_deletes_own_tasks() {
    let state = setup_test_state();
    let project_id = ProjectId::new();

    // Create two sessions
    let session_a = IdeationSession::new(project_id.clone());
    let created_a = state
        .ideation_session_repo
        .create(session_a)
        .await
        .expect("Failed to create session A");

    let session_b = IdeationSession::new(project_id.clone());
    let created_b = state
        .ideation_session_repo
        .create(session_b)
        .await
        .expect("Failed to create session B");

    // Create tasks: 2 for session A, 1 for session B
    let mut task_a1 =
        crate::domain::entities::Task::new(project_id.clone(), "A-Task 1".to_string());
    task_a1.ideation_session_id = Some(created_a.id.clone());
    let mut task_a2 =
        crate::domain::entities::Task::new(project_id.clone(), "A-Task 2".to_string());
    task_a2.ideation_session_id = Some(created_a.id.clone());
    let mut task_b1 =
        crate::domain::entities::Task::new(project_id.clone(), "B-Task 1".to_string());
    task_b1.ideation_session_id = Some(created_b.id.clone());

    state.task_repo.create(task_a1).await.unwrap();
    state.task_repo.create(task_a2).await.unwrap();
    let tb1 = state.task_repo.create(task_b1).await.unwrap();

    // Cascade delete session A's tasks only
    let tasks_a = state
        .task_repo
        .get_by_ideation_session(&created_a.id)
        .await
        .unwrap();
    assert_eq!(tasks_a.len(), 2);

    for task in &tasks_a {
        state.task_repo.delete(&task.id).await.unwrap();
    }
    state
        .ideation_session_repo
        .delete(&created_a.id)
        .await
        .unwrap();

    // Session B's task should be untouched
    let tasks_b = state
        .task_repo
        .get_by_ideation_session(&created_b.id)
        .await
        .unwrap();
    assert_eq!(tasks_b.len(), 1);
    assert_eq!(tasks_b[0].title, "B-Task 1");

    // Session B's task still retrievable by ID
    assert!(state.task_repo.get_by_id(&tb1.id).await.unwrap().is_some());
}

// ========================================================================
// ExecutionPlan Integration Tests (Phase 46-48)
// ========================================================================

#[tokio::test]
async fn test_execution_plan_created_and_stored() {
    use crate::domain::entities::{ExecutionPlan, ExecutionPlanStatus, IdeationSession};

    let state = setup_test_state();
    let project_id = ProjectId::new();

    // ExecutionPlan has FK on session_id — create session first
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    let plan = ExecutionPlan::new(created_session.id.clone());
    let plan_id = plan.id.clone();

    let created = state
        .execution_plan_repo
        .create(plan)
        .await
        .expect("Failed to create execution plan");

    assert_eq!(created.id, plan_id);
    assert_eq!(created.session_id, created_session.id);
    assert_eq!(created.status, ExecutionPlanStatus::Active);
}

#[tokio::test]
async fn test_task_execution_plan_id_persists() {
    use crate::domain::entities::{ExecutionPlan, ExecutionPlanId, IdeationSession};

    let state = setup_test_state();
    let project_id = ProjectId::new();

    // ExecutionPlan has FK on session_id — create session first
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");
    let session_id = created_session.id.clone();

    // Create an ExecutionPlan
    let plan = ExecutionPlan::new(session_id.clone());
    let exec_plan_id: ExecutionPlanId = plan.id.clone();
    state
        .execution_plan_repo
        .create(plan)
        .await
        .expect("Failed to create execution plan");

    // Create a task with execution_plan_id set
    let mut task = crate::domain::entities::Task::new(project_id.clone(), "EP Task".to_string());
    task.ideation_session_id = Some(session_id.clone());
    task.execution_plan_id = Some(exec_plan_id.clone());

    let created = state
        .task_repo
        .create(task.clone())
        .await
        .expect("Failed to create task");

    assert_eq!(created.execution_plan_id, Some(exec_plan_id.clone()));

    // Retrieve and verify field persists
    let fetched = state
        .task_repo
        .get_by_id(&task.id)
        .await
        .expect("get_by_id failed")
        .expect("task not found");

    assert_eq!(fetched.execution_plan_id, Some(exec_plan_id));
}

#[tokio::test]
async fn test_plan_branch_execution_plan_id_persists() {
    use crate::domain::entities::{ArtifactId, ExecutionPlan, ExecutionPlanId, IdeationSession, PlanBranch};

    let state = setup_test_state();
    let project_id = ProjectId::new();

    // ExecutionPlan has FK on session_id — create session first
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");
    let session_id = created_session.id.clone();

    // Create an ExecutionPlan
    let plan = ExecutionPlan::new(session_id.clone());
    let exec_plan_id: ExecutionPlanId = plan.id.clone();
    state
        .execution_plan_repo
        .create(plan)
        .await
        .expect("Failed to create execution plan");

    // Create plan branch with execution_plan_id set
    let mut branch = PlanBranch::new(
        ArtifactId::from_string("art-123"),
        session_id.clone(),
        project_id.clone(),
        format!("ralphx/test/plan-{}", &exec_plan_id.as_str()[..8]),
        "main".to_string(),
    );
    branch.execution_plan_id = Some(exec_plan_id.clone());

    state
        .plan_branch_repo
        .create(branch)
        .await
        .expect("Failed to create plan branch");

    // Lookup by execution_plan_id
    let found = state
        .plan_branch_repo
        .get_by_execution_plan_id(&exec_plan_id)
        .await
        .expect("get_by_execution_plan_id failed");

    assert!(found.is_some(), "Branch not found by execution_plan_id");
    let branch = found.unwrap();
    assert_eq!(branch.execution_plan_id, Some(exec_plan_id));
    assert_eq!(branch.session_id, session_id);
}

#[tokio::test]
async fn test_two_execution_plans_same_session_have_unique_ids() {
    use crate::domain::entities::{ExecutionPlan, IdeationSession};

    let state = setup_test_state();
    let project_id = ProjectId::new();

    // ExecutionPlan has FK on session_id — create session first
    let session = IdeationSession::new(project_id.clone());
    let created_session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");
    let session_id = created_session.id.clone();

    let plan1 = ExecutionPlan::new(session_id.clone());
    let plan2 = ExecutionPlan::new(session_id.clone());

    let id1 = plan1.id.clone();
    let id2 = plan2.id.clone();

    state
        .execution_plan_repo
        .create(plan1)
        .await
        .expect("Failed to create plan1");
    state
        .execution_plan_repo
        .create(plan2)
        .await
        .expect("Failed to create plan2");

    assert_ne!(id1, id2, "Each re-accept must produce a unique ExecutionPlan ID");
}

#[tokio::test]
async fn test_get_by_execution_plan_id_returns_none_when_absent() {
    use crate::domain::entities::ExecutionPlanId;

    let state = setup_test_state();
    let nonexistent_id = ExecutionPlanId::new();

    let result = state
        .plan_branch_repo
        .get_by_execution_plan_id(&nonexistent_id)
        .await
        .expect("get_by_execution_plan_id should not error");

    assert!(result.is_none());
}

// ============================================================================
// apply_proposals_core regression tests
// ============================================================================

/// Helper: create a project and session with N proposals, return (project_id, session, proposal_ids)
async fn setup_session_with_proposals(
    state: &AppState,
    proposal_count: usize,
) -> (
    ProjectId,
    crate::domain::entities::IdeationSession,
    Vec<String>,
) {
    use crate::domain::entities::{Project, ProposalCategory, Priority};

    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    let project = state
        .project_repo
        .create(project)
        .await
        .expect("Failed to create project");

    let session = IdeationSession::new(project.id.clone());
    let session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    let mut ids = Vec::new();
    for i in 0..proposal_count {
        let proposal = TaskProposal::new(
            session.id.clone(),
            format!("Proposal {}", i + 1),
            ProposalCategory::Feature,
            Priority::Medium,
        );
        let p = state
            .task_proposal_repo
            .create(proposal)
            .await
            .expect("Failed to create proposal");
        ids.push(p.id.as_str().to_string());
    }

    (project.id, session, ids)
}

#[tokio::test]
async fn test_apply_proposals_core_creates_tasks_with_ready_status() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;
    use crate::domain::entities::InternalStatus;

    let state = setup_test_state();
    let (project_id, session, proposal_ids) = setup_session_with_proposals(&state, 2).await;

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids: proposal_ids.clone(),
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("apply_proposals_core should succeed");

    assert_eq!(result.created_task_ids.len(), 2, "Should create 2 tasks");
    assert_eq!(result.dependencies_created, 0);
    assert!(result.warnings.is_empty());
    assert_eq!(result.project_id, project_id.as_str());
    assert_eq!(result.session_id, session.id.as_str());
    assert!(result.any_ready_tasks, "Tasks with no blockers should be Ready");

    // Verify tasks are actually Ready in the repo
    for task_id_str in &result.created_task_ids {
        let task_id = crate::domain::entities::TaskId::from_string(task_id_str.clone());
        let task = state
            .task_repo
            .get_by_id(&task_id)
            .await
            .expect("repo error")
            .expect("task should exist");
        assert_eq!(
            task.internal_status,
            InternalStatus::Ready,
            "Task without blockers should be Ready"
        );
        assert_eq!(task.ideation_session_id, Some(session.id.clone()));
    }
}

#[tokio::test]
async fn test_apply_proposals_core_session_converts_to_accepted() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;

    let state = setup_test_state();
    let (_project_id, session, proposal_ids) = setup_session_with_proposals(&state, 1).await;

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids,
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("apply_proposals_core should succeed");

    assert!(result.session_converted, "All proposals applied — session should convert");

    // Verify session status is Accepted in repo
    let updated_session = state
        .ideation_session_repo
        .get_by_id(&session.id)
        .await
        .expect("repo error")
        .expect("session should exist");
    assert_eq!(updated_session.status, IdeationSessionStatus::Accepted);
}

#[tokio::test]
async fn test_apply_proposals_core_partial_apply_does_not_convert_session() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;

    let state = setup_test_state();
    let (_project_id, session, proposal_ids) = setup_session_with_proposals(&state, 2).await;

    // Only apply 1 of 2 proposals
    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids: vec![proposal_ids[0].clone()],
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("apply_proposals_core should succeed");

    assert!(!result.session_converted, "Partial apply should not convert session");

    // Session should still be Active
    let updated_session = state
        .ideation_session_repo
        .get_by_id(&session.id)
        .await
        .expect("repo error")
        .expect("session should exist");
    assert_eq!(updated_session.status, IdeationSessionStatus::Active);
}

#[tokio::test]
async fn test_apply_proposals_core_idempotency_guard() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;
    use crate::domain::entities::ExecutionPlan;

    let state = setup_test_state();
    let (_project_id, session, proposal_ids) = setup_session_with_proposals(&state, 2).await;

    // Pre-seed an active ExecutionPlan for this session to simulate a race condition
    // (two simultaneous accepts before either updates the session status).
    let existing_plan = ExecutionPlan::new(session.id.clone());
    state
        .execution_plan_repo
        .create(existing_plan)
        .await
        .expect("Failed to create pre-existing execution plan");

    // Apply should hit the idempotency guard and return early
    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids,
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("Idempotency guard should return Ok, not error");

    assert_eq!(
        result.created_task_ids.len(),
        0,
        "Idempotency guard: no tasks created when plan already exists"
    );
    assert!(
        !result.warnings.is_empty(),
        "Idempotency guard: should emit a warning"
    );
    assert!(
        result.warnings[0].contains("already active"),
        "Warning should mention existing plan"
    );
}

#[tokio::test]
async fn test_apply_proposals_core_rejects_inactive_session() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;
    use crate::error::AppError;

    let state = setup_test_state();
    let (_project_id, session, proposal_ids) = setup_session_with_proposals(&state, 1).await;

    // Archive the session so it is no longer Active
    state
        .ideation_session_repo
        .update_status(&session.id, IdeationSessionStatus::Archived)
        .await
        .expect("Failed to archive session");

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids,
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let err = apply_proposals_core(&state, input)
        .await
        .expect_err("Should fail for inactive session");

    assert!(
        matches!(err, AppError::Validation(_)),
        "Expected Validation error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_apply_proposals_core_rejects_unknown_proposals() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;
    use crate::error::AppError;

    let state = setup_test_state();
    let (_project_id, session, _) = setup_session_with_proposals(&state, 1).await;

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids: vec!["nonexistent-proposal-id".to_string()],
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let err = apply_proposals_core(&state, input)
        .await
        .expect_err("Should fail when proposals not found");

    assert!(
        matches!(err, AppError::Validation(_)),
        "Expected Validation error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_apply_proposals_core_result_contains_context_fields() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;

    let state = setup_test_state();
    let (project_id, session, proposal_ids) = setup_session_with_proposals(&state, 2).await;

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids,
        target_column: "auto".to_string(),
        preserve_dependencies: false,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("apply_proposals_core should succeed");

    // Verify context fields for Tauri side effects
    assert_eq!(result.project_id, project_id.as_str(), "project_id must match");
    assert_eq!(result.session_id, session.id.as_str(), "session_id must match");
    assert_eq!(result.proposal_titles.len(), 2, "proposal_titles should contain all applied titles");
    assert!(!result.is_user_title, "New session has no user title");
    assert!(result.execution_plan_id.is_some(), "execution_plan_id must be set");
}

#[tokio::test]
async fn test_apply_proposals_core_preserves_dependencies() {
    use crate::commands::ideation_commands::ideation_commands_apply::apply_proposals_core;
    use crate::commands::ideation_commands::ApplyProposalsInput;
    use crate::domain::entities::{InternalStatus, Priority, ProposalCategory, Project};

    let state = setup_test_state();

    let project = Project::new("Dep Test".to_string(), "/tmp/dep".to_string());
    let project = state
        .project_repo
        .create(project)
        .await
        .expect("Failed to create project");

    let session = IdeationSession::new(project.id.clone());
    let session = state
        .ideation_session_repo
        .create(session)
        .await
        .expect("Failed to create session");

    // Create p1 (blocker) and p2 (depends on p1)
    let p1 = state
        .task_proposal_repo
        .create(TaskProposal::new(
            session.id.clone(),
            "Blocker Task",
            ProposalCategory::Feature,
            Priority::High,
        ))
        .await
        .expect("Failed to create p1");

    let p2 = state
        .task_proposal_repo
        .create(TaskProposal::new(
            session.id.clone(),
            "Dependent Task",
            ProposalCategory::Feature,
            Priority::Medium,
        ))
        .await
        .expect("Failed to create p2");

    // p2 depends on p1
    state
        .proposal_dependency_repo
        .add_dependency(&p2.id, &p1.id, None, Some("manual"))
        .await
        .expect("Failed to add proposal dependency");

    let input = ApplyProposalsInput {
        session_id: session.id.as_str().to_string(),
        proposal_ids: vec![
            p1.id.as_str().to_string(),
            p2.id.as_str().to_string(),
        ],
        target_column: "auto".to_string(),
        preserve_dependencies: true,
        use_feature_branch: Some(false),
    };

    let result = apply_proposals_core(&state, input)
        .await
        .expect("apply_proposals_core should succeed");

    assert_eq!(result.created_task_ids.len(), 2);
    assert_eq!(result.dependencies_created, 1, "One dependency should be created");

    // Verify statuses: p1 task → Ready, p2 task → Blocked
    let tasks = state
        .task_repo
        .get_by_project(&project.id)
        .await
        .expect("Failed to get tasks");

    let blocker_task = tasks.iter().find(|t| t.title == "Blocker Task").expect("Blocker task not found");
    let dependent_task = tasks.iter().find(|t| t.title == "Dependent Task").expect("Dependent task not found");

    assert_eq!(blocker_task.internal_status, InternalStatus::Ready, "Blocker task has no blockers → Ready");
    assert_eq!(dependent_task.internal_status, InternalStatus::Blocked, "Dependent task has blocker → Blocked");
    assert!(dependent_task.blocked_reason.is_some(), "Blocked task should have a reason");
}
