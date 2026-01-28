// Ideation commands module - aggregates all ideation-related submodules

mod ideation_commands_types;
mod ideation_commands_session;
mod ideation_commands_proposals;
mod ideation_commands_dependencies;
mod ideation_commands_apply;
mod ideation_commands_chat;
mod ideation_commands_orchestrator;

// Re-export all types
pub use ideation_commands_types::*;

// Re-export all commands
pub use ideation_commands_session::*;
pub use ideation_commands_proposals::*;
pub use ideation_commands_dependencies::*;
pub use ideation_commands_apply::*;
pub use ideation_commands_chat::*;
pub use ideation_commands_orchestrator::*;

// Re-export helper function for tests
pub use ideation_commands_dependencies::build_dependency_graph;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{
        ChatMessage, IdeationSession, IdeationSessionId, IdeationSessionStatus,
        Priority, ProjectId, TaskCategory, TaskProposal, TaskProposalId,
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
        let created = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        assert_eq!(created.project_id, project_id);
        assert!(created.title.is_none());
        assert_eq!(created.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_create_ideation_session_with_title() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
        let created = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        assert_eq!(created.project_id, project_id);
        assert_eq!(created.title, Some("Test Session".to_string()));
        assert_eq!(created.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_get_ideation_session_returns_none_for_nonexistent() {
        let state = setup_test_state();
        let id = IdeationSessionId::new();

        let result = state.ideation_session_repo.get_by_id(&id).await.expect("Failed to get ideation session by id in test");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_ideation_session_returns_existing() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        let session = IdeationSession::new(project_id);
        let created = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

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
        let created = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

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
        let created = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

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
            crate::domain::entities::TaskCategory::Feature,
            crate::domain::entities::Priority::High,
        );
        state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");

        // Create message for session
        let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello");
        state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

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
            crate::domain::entities::TaskCategory::Feature,
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create proposal
        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::High,
        );
        let created = state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");

        assert_eq!(created.title, "Test Proposal");
        assert_eq!(created.category, TaskCategory::Feature);
        assert_eq!(created.suggested_priority, Priority::High);
    }

    #[tokio::test]
    async fn test_get_task_proposal_returns_none_for_nonexistent() {
        let state = setup_test_state();
        let id = TaskProposalId::new();

        let result = state.task_proposal_repo.get_by_id(&id).await.expect("Failed to get task proposal by id in test");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_session_proposals() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create proposals
        for i in 0..3 {
            let proposal = TaskProposal::new(
                created_session.id.clone(),
                format!("Proposal {}", i),
                TaskCategory::Feature,
                Priority::Medium,
            );
            state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Original Title",
            TaskCategory::Feature,
            Priority::Low,
        );
        let created = state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");

        // Update proposal
        let mut updated = created.clone();
        updated.title = "Updated Title".to_string();
        updated.user_modified = true;

        state.task_proposal_repo.update(&updated).await.expect("Failed to update task proposal in test");

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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "To Delete",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let created = state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");

        // Delete proposal
        state.task_proposal_repo.delete(&created.id).await.expect("Failed to delete task proposal in test");

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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal = TaskProposal::new(
            created_session.id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let created = state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");

        // Initial state should be selected (true)
        assert!(created.selected);

        // Toggle to false
        state
            .task_proposal_repo
            .update_selection(&created.id, false)
            .await
            .expect("Failed to update selection in test");

        let retrieved = state
            .task_proposal_repo
            .get_by_id(&created.id)
            .await
            .expect("Failed to get by id in test")
            .expect("Expected to find entity");
        assert!(!retrieved.selected);
    }

    #[tokio::test]
    async fn test_reorder_proposals() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create 3 proposals
        let mut ids = Vec::new();
        for i in 0..3 {
            let proposal = TaskProposal::new(
                created_session.id.clone(),
                format!("Proposal {}", i),
                TaskCategory::Feature,
                Priority::Medium,
            );
            let created = state.task_proposal_repo.create(proposal).await.expect("Failed to create task proposal in test");
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
            TaskCategory::Feature,
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.expect("Failed to create task proposal in test");
        let p2 = state.task_proposal_repo.create(proposal2).await.expect("Failed to create task proposal in test");

        // Add dependency: p1 depends on p2
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.expect("Failed to create task proposal in test");
        let p2 = state.task_proposal_repo.create(proposal2).await.expect("Failed to create task proposal in test");

        // Add then remove dependency
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let proposal1 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 1",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal2 = TaskProposal::new(
            created_session.id.clone(),
            "Proposal 2",
            TaskCategory::Feature,
            Priority::Medium,
        );

        let p1 = state.task_proposal_repo.create(proposal1).await.expect("Failed to create task proposal in test");
        let p2 = state.task_proposal_repo.create(proposal2).await.expect("Failed to create task proposal in test");

        // p1 depends on p2, so p2 should have p1 as a dependent
        state
            .proposal_dependency_repo
            .add_dependency(&p1.id, &p2.id)
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

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

        let t1 = state.task_repo.create(task1).await.expect("Failed to create task in test");
        let t2 = state.task_repo.create(task2).await.expect("Failed to create task in test");

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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Send a message
        let message = ChatMessage::user_in_session(created_session.id.clone(), "Hello world");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

        assert_eq!(created.content, "Hello world");
        assert_eq!(created.session_id, Some(created_session.id));
    }

    #[tokio::test]
    async fn test_send_chat_message_to_project() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Send a message to project
        let message = ChatMessage::user_in_project(project_id.clone(), "Project message");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

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
        let created_task = state.task_repo.create(task).await.expect("Failed to create task in test");

        // Send a message about the task
        let message = ChatMessage::user_about_task(created_task.id.clone(), "Task message");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

        assert_eq!(created.content, "Task message");
        assert_eq!(created.task_id, Some(created_task.id));
    }

    #[tokio::test]
    async fn test_get_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Send multiple messages
        for i in 1..=3 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
        let created_task = state.task_repo.create(task).await.expect("Failed to create task in test");

        // Send messages about the task
        for i in 1..=2 {
            let message =
                ChatMessage::user_about_task(created_task.id.clone(), format!("Task message {}", i));
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        let message = ChatMessage::user_in_session(created_session.id.clone(), "To delete");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

        // Delete the message
        state.chat_message_repo.delete(&created.id).await.expect("Failed to delete chat message in test");

        // Verify it's gone
        let result = state.chat_message_repo.get_by_id(&created.id).await.expect("Failed to get chat message by id in test");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_session_messages() {
        let state = setup_test_state();
        let project_id = ProjectId::new();

        // Create session
        let session = IdeationSession::new(project_id);
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create multiple messages
        for i in 1..=3 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create messages
        for i in 1..=5 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create 5 messages
        for i in 1..=5 {
            let message =
                ChatMessage::user_in_session(created_session.id.clone(), format!("Message {}", i));
            state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");
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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create orchestrator message
        let message =
            ChatMessage::orchestrator_in_session(created_session.id.clone(), "AI response");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

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
        let created_session = state.ideation_session_repo.create(session).await.expect("Failed to create ideation session in test");

        // Create system message
        let message = ChatMessage::system_in_session(created_session.id.clone(), "Session started");
        let created = state.chat_message_repo.create(message).await.expect("Failed to create chat message in test");

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

        assert_eq!(settings.plan_mode, crate::domain::ideation::IdeationPlanMode::Optional);
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
        };

        // Update settings
        let updated = state
            .ideation_settings_repo
            .update_settings(&custom_settings)
            .await
            .expect("Failed to update ideation settings in test");

        assert_eq!(updated.plan_mode, crate::domain::ideation::IdeationPlanMode::Required);
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

        assert_eq!(retrieved.plan_mode, crate::domain::ideation::IdeationPlanMode::Parallel);
        assert!(!retrieved.require_plan_approval);
        assert!(retrieved.suggest_plans_for_complex);
        assert!(!retrieved.auto_link_proposals);
    }
}
