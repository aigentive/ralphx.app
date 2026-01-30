use super::*;
use crate::domain::entities::{ArtifactId, Complexity, Priority, TaskCategory};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

// ============================================================================
// Mock Repositories
// ============================================================================

struct MockTaskProposalRepository {
        proposals: Mutex<Vec<TaskProposal>>,
        updated_priorities: Mutex<Vec<(TaskProposalId, PriorityAssessment)>>,
    }

    impl MockTaskProposalRepository {
        fn with_proposals(proposals: Vec<TaskProposal>) -> Self {
            Self {
                proposals: Mutex::new(proposals),
                updated_priorities: Mutex::new(Vec::new()),
            }
        }

        fn get_updated_priorities(&self) -> Vec<(TaskProposalId, PriorityAssessment)> {
            self.updated_priorities.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl TaskProposalRepository for MockTaskProposalRepository {
        async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
            self.proposals.lock().unwrap().push(proposal.clone());
            Ok(proposal)
        }

        async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .find(|p| &p.id == id)
                .cloned())
        }

        async fn get_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .filter(|p| &p.session_id == session_id)
                .cloned()
                .collect())
        }

        async fn update(&self, _proposal: &TaskProposal) -> AppResult<()> {
            Ok(())
        }

        async fn update_priority(
            &self,
            id: &TaskProposalId,
            assessment: &PriorityAssessment,
        ) -> AppResult<()> {
            self.updated_priorities
                .lock()
                .unwrap()
                .push((id.clone(), assessment.clone()));
            Ok(())
        }

        async fn update_selection(&self, _id: &TaskProposalId, _selected: bool) -> AppResult<()> {
            Ok(())
        }

        async fn set_created_task_id(
            &self,
            _id: &TaskProposalId,
            _task_id: &crate::domain::entities::TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &TaskProposalId) -> AppResult<()> {
            Ok(())
        }

        async fn reorder(
            &self,
            _session_id: &IdeationSessionId,
            _proposal_ids: Vec<TaskProposalId>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .filter(|p| &p.session_id == session_id && p.selected)
                .cloned()
                .collect())
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .filter(|p| &p.session_id == session_id)
                .count() as u32)
        }

        async fn count_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .filter(|p| &p.session_id == session_id && p.selected)
                .count() as u32)
        }

        async fn get_by_plan_artifact_id(&self, artifact_id: &ArtifactId) -> AppResult<Vec<TaskProposal>> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.plan_artifact_id.as_ref() == Some(artifact_id))
                .cloned()
                .collect())
        }
    }

    struct MockProposalDependencyRepository {
        // Map from proposal_id to set of depends_on_ids
        dependencies: Mutex<HashMap<TaskProposalId, HashSet<TaskProposalId>>>,
    }

    impl MockProposalDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: Mutex::new(HashMap::new()),
            }
        }

        fn with_dependencies(deps: Vec<(TaskProposalId, TaskProposalId)>) -> Self {
            let mut map: HashMap<TaskProposalId, HashSet<TaskProposalId>> = HashMap::new();
            for (from, to) in deps {
                map.entry(from).or_default().insert(to);
            }
            Self {
                dependencies: Mutex::new(map),
            }
        }
    }

    #[async_trait]
    impl ProposalDependencyRepository for MockProposalDependencyRepository {
        async fn add_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
            _reason: Option<&str>,
        ) -> AppResult<()> {
            self.dependencies
                .lock()
                .unwrap()
                .entry(proposal_id.clone())
                .or_default()
                .insert(depends_on_id.clone());
            Ok(())
        }

        async fn remove_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            if let Some(set) = self.dependencies.lock().unwrap().get_mut(proposal_id) {
                set.remove(depends_on_id);
            }
            Ok(())
        }

        async fn get_dependencies(
            &self,
            proposal_id: &TaskProposalId,
        ) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .get(proposal_id)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default())
        }

        async fn get_dependents(
            &self,
            proposal_id: &TaskProposalId,
        ) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter_map(|(id, deps)| {
                    if deps.contains(proposal_id) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }

        async fn get_all_for_session(
            &self,
            _session_id: &IdeationSessionId,
        ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>)>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .flat_map(|(from, tos)| tos.iter().map(|to| (from.clone(), to.clone(), None)))
                .collect())
        }

        async fn would_create_cycle(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<bool> {
            if proposal_id == depends_on_id {
                return Ok(true);
            }
            let deps = self.dependencies.lock().unwrap();
            if let Some(dep_set) = deps.get(depends_on_id) {
                if dep_set.contains(proposal_id) {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
            let mut deps = self.dependencies.lock().unwrap();
            deps.remove(proposal_id);
            for set in deps.values_mut() {
                set.remove(proposal_id);
            }
            Ok(())
        }

        async fn clear_session_dependencies(&self, _session_id: &IdeationSessionId) -> AppResult<()> {
            self.dependencies.lock().unwrap().clear();
            Ok(())
        }

        async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .get(proposal_id)
                .map(|set| set.len() as u32)
                .unwrap_or(0))
        }

        async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, deps)| deps.contains(proposal_id))
                .count() as u32)
        }
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    fn create_test_proposal(session_id: &IdeationSessionId, title: &str) -> TaskProposal {
        TaskProposal::new(session_id.clone(), title, TaskCategory::Feature, Priority::Medium)
    }

    fn create_proposal_with_complexity(
        session_id: &IdeationSessionId,
        title: &str,
        complexity: Complexity,
    ) -> TaskProposal {
        let mut proposal =
            TaskProposal::new(session_id.clone(), title, TaskCategory::Feature, Priority::Medium);
        proposal.estimated_complexity = complexity;
        proposal
    }

    fn create_service(
        proposals: Vec<TaskProposal>,
        dependencies: Vec<(TaskProposalId, TaskProposalId)>,
    ) -> PriorityService<MockTaskProposalRepository, MockProposalDependencyRepository> {
        let proposal_repo = Arc::new(MockTaskProposalRepository::with_proposals(proposals));
        let dep_repo = Arc::new(MockProposalDependencyRepository::with_dependencies(
            dependencies,
        ));
        PriorityService::new(proposal_repo, dep_repo)
    }

    // ============================================================================
    // Dependency Factor Tests
    // ============================================================================

    #[test]
    fn test_dependency_factor_zero_blocks() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_dependency_factor(0);

        assert_eq!(factor.score, 0);
        assert_eq!(factor.blocks_count, 0);
        assert_eq!(factor.reason, "Does not block other tasks");
    }

    #[test]
    fn test_dependency_factor_one_block() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_dependency_factor(1);

        assert_eq!(factor.score, 10);
        assert_eq!(factor.blocks_count, 1);
        assert_eq!(factor.reason, "Blocks 1 other task");
    }

    #[test]
    fn test_dependency_factor_two_blocks() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_dependency_factor(2);

        assert_eq!(factor.score, 18);
        assert_eq!(factor.blocks_count, 2);
        assert_eq!(factor.reason, "Blocks 2 other tasks");
    }

    #[test]
    fn test_dependency_factor_three_blocks() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_dependency_factor(3);

        assert_eq!(factor.score, 24);
        assert_eq!(factor.blocks_count, 3);
    }

    #[test]
    fn test_dependency_factor_max_blocks() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_dependency_factor(10);

        assert_eq!(factor.score, 30); // Max score
        assert_eq!(factor.blocks_count, 10);
    }

    // ============================================================================
    // Critical Path Factor Tests
    // ============================================================================

    #[test]
    fn test_critical_path_factor_not_on_path() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_critical_path_factor(false, 0);

        assert_eq!(factor.score, 0);
        assert!(!factor.is_on_critical_path);
        assert_eq!(factor.reason, "Not on critical path");
    }

    #[test]
    fn test_critical_path_factor_path_length_1() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_critical_path_factor(true, 1);

        assert_eq!(factor.score, 10);
        assert!(factor.is_on_critical_path);
        assert_eq!(factor.path_length, 1);
    }

    #[test]
    fn test_critical_path_factor_path_length_2() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_critical_path_factor(true, 2);

        assert_eq!(factor.score, 15);
        assert!(factor.is_on_critical_path);
    }

    #[test]
    fn test_critical_path_factor_path_length_3() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_critical_path_factor(true, 3);

        assert_eq!(factor.score, 20);
    }

    #[test]
    fn test_critical_path_factor_long_path() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_critical_path_factor(true, 10);

        assert_eq!(factor.score, 25); // Max score
    }

    // ============================================================================
    // Business Value Factor Tests
    // ============================================================================

    #[test]
    fn test_business_value_factor_no_keywords() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_business_value_factor("Implement user profile page");

        assert_eq!(factor.score, 10); // Default score
        assert!(factor.keywords.is_empty());
    }

    #[test]
    fn test_business_value_factor_critical_keyword() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_business_value_factor("This is critical for launch");

        assert_eq!(factor.score, 20); // Max for critical
        assert!(factor.keywords.contains(&"critical".to_string()));
    }

    #[test]
    fn test_business_value_factor_urgent_keyword() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_business_value_factor("URGENT: fix this ASAP");

        assert_eq!(factor.score, 20);
        assert!(factor.keywords.iter().any(|k| k == "urgent" || k == "asap"));
    }

    #[test]
    fn test_business_value_factor_high_keywords() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_business_value_factor("MVP core feature essential");

        assert_eq!(factor.score, 15);
        assert!(!factor.keywords.is_empty());
    }

    #[test]
    fn test_business_value_factor_low_keywords() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_business_value_factor("Nice to have feature for later");

        assert_eq!(factor.score, 5);
        assert!(!factor.keywords.is_empty());
    }

    // ============================================================================
    // Complexity Factor Tests
    // ============================================================================

    #[test]
    fn test_complexity_factor_trivial() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_complexity_factor(Complexity::Trivial);

        assert_eq!(factor.score, 15); // Max for trivial (quick wins)
        assert_eq!(factor.complexity, Complexity::Trivial);
    }

    #[test]
    fn test_complexity_factor_simple() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_complexity_factor(Complexity::Simple);

        assert_eq!(factor.score, 12);
    }

    #[test]
    fn test_complexity_factor_moderate() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_complexity_factor(Complexity::Moderate);

        assert_eq!(factor.score, 9);
    }

    #[test]
    fn test_complexity_factor_complex() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_complexity_factor(Complexity::Complex);

        assert_eq!(factor.score, 5);
    }

    #[test]
    fn test_complexity_factor_very_complex() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_complexity_factor(Complexity::VeryComplex);

        assert_eq!(factor.score, 2); // Min for very complex
    }

    // ============================================================================
    // User Hint Factor Tests
    // ============================================================================

    #[test]
    fn test_user_hint_factor_no_hints() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_user_hint_factor("Regular task description");

        assert_eq!(factor.score, 0);
        assert!(factor.hints.is_empty());
    }

    #[test]
    fn test_user_hint_factor_urgent_hint() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_user_hint_factor("This is urgent");

        assert!(factor.score > 0);
        assert!(factor.hints.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_user_hint_factor_multiple_hints() {
        let service = create_service(vec![], vec![]);
        let factor = service.calculate_user_hint_factor("Do this immediately, it's a blocker and urgent");

        assert!(factor.score >= 6); // At least 2 hints * 3 points each
        assert!(factor.hints.len() >= 2);
    }

    #[test]
    fn test_user_hint_factor_max_score() {
        let service = create_service(vec![], vec![]);
        let factor =
            service.calculate_user_hint_factor("urgent asap immediately now today deadline blocker");

        assert_eq!(factor.score, 10); // Capped at max
    }

    // ============================================================================
    // Score to Priority Tests
    // ============================================================================

    #[test]
    fn test_score_to_priority_critical() {
        let service = create_service(vec![], vec![]);

        assert_eq!(service.score_to_priority(100), Priority::Critical);
        assert_eq!(service.score_to_priority(80), Priority::Critical);
    }

    #[test]
    fn test_score_to_priority_high() {
        let service = create_service(vec![], vec![]);

        assert_eq!(service.score_to_priority(79), Priority::High);
        assert_eq!(service.score_to_priority(60), Priority::High);
    }

    #[test]
    fn test_score_to_priority_medium() {
        let service = create_service(vec![], vec![]);

        assert_eq!(service.score_to_priority(59), Priority::Medium);
        assert_eq!(service.score_to_priority(40), Priority::Medium);
    }

    #[test]
    fn test_score_to_priority_low() {
        let service = create_service(vec![], vec![]);

        assert_eq!(service.score_to_priority(39), Priority::Low);
        assert_eq!(service.score_to_priority(0), Priority::Low);
    }

    // ============================================================================
    // Build Dependency Graph Tests
    // ============================================================================

    #[tokio::test]
    async fn test_build_graph_empty_session() {
        let service = create_service(vec![], vec![]);
        let session_id = IdeationSessionId::new();

        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        assert!(graph.is_empty());
        assert!(!graph.has_cycles);
        assert!(graph.critical_path.is_empty());
    }

    #[tokio::test]
    async fn test_build_graph_single_proposal() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id, "Test task");

        let service = create_service(vec![proposal.clone()], vec![]);

        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.has_cycles);
    }

    #[tokio::test]
    async fn test_build_graph_linear_chain() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");
        let p3 = create_test_proposal(&session_id, "Task 3");

        // p2 depends on p1, p3 depends on p2
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p2.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert!(!graph.has_cycles);
        assert_eq!(graph.critical_path_length(), 3);
    }

    #[tokio::test]
    async fn test_build_graph_detects_cycles() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");
        let p3 = create_test_proposal(&session_id, "Task 3");

        // Circular: p1 -> p2 -> p3 -> p1
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p3.id.clone()),
            (p3.id.clone(), p1.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        assert!(graph.has_cycles);
        assert!(graph.critical_path.is_empty()); // Can't compute with cycles
    }

    // ============================================================================
    // Assess Priority Tests
    // ============================================================================

    #[tokio::test]
    async fn test_assess_priority_basic() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id, "Test task");

        let service = create_service(vec![proposal.clone()], vec![]);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let assessment = service.assess_priority(&proposal, &graph).await.unwrap();

        assert_eq!(assessment.proposal_id, proposal.id);
        assert!(assessment.priority_score >= 0 && assessment.priority_score <= 100);
    }

    #[tokio::test]
    async fn test_assess_priority_with_blockers() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Foundation task");
        let p2 = create_test_proposal(&session_id, "Dependent task 1");
        let p3 = create_test_proposal(&session_id, "Dependent task 2");

        // p2 and p3 depend on p1, so p1 blocks 2 tasks
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p1.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let assessment = service.assess_priority(&p1, &graph).await.unwrap();

        // p1 blocks 2 tasks, so dependency factor should be 18
        assert_eq!(assessment.factors.dependency_factor.score, 18);
        assert_eq!(assessment.factors.dependency_factor.blocks_count, 2);
    }

    #[tokio::test]
    async fn test_assess_priority_critical_keywords() {
        let session_id = IdeationSessionId::new();
        let mut proposal = create_test_proposal(&session_id, "Critical blocker for launch");
        proposal.description = Some("This is urgent and must be done ASAP".to_string());

        let service = create_service(vec![proposal.clone()], vec![]);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let assessment = service.assess_priority(&proposal, &graph).await.unwrap();

        // Should have high business value and user hint scores
        assert_eq!(assessment.factors.business_value_factor.score, 20);
        assert!(assessment.factors.user_hint_factor.score > 0);
    }

    #[tokio::test]
    async fn test_assess_priority_complexity_affects_score() {
        let session_id = IdeationSessionId::new();
        let trivial = create_proposal_with_complexity(&session_id, "Simple fix", Complexity::Trivial);
        let complex =
            create_proposal_with_complexity(&session_id, "Major refactor", Complexity::VeryComplex);

        let service = create_service(vec![trivial.clone(), complex.clone()], vec![]);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let trivial_assessment = service.assess_priority(&trivial, &graph).await.unwrap();
        let complex_assessment = service.assess_priority(&complex, &graph).await.unwrap();

        assert!(
            trivial_assessment.factors.complexity_factor.score
                > complex_assessment.factors.complexity_factor.score
        );
    }

    // ============================================================================
    // Assess All Priorities Tests
    // ============================================================================

    #[tokio::test]
    async fn test_assess_all_priorities_empty() {
        let session_id = IdeationSessionId::new();
        let service = create_service(vec![], vec![]);

        let assessments = service.assess_all_priorities(&session_id).await.unwrap();

        assert!(assessments.is_empty());
    }

    #[tokio::test]
    async fn test_assess_all_priorities_multiple() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");
        let p3 = create_test_proposal(&session_id, "Task 3");

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], vec![]);

        let assessments = service.assess_all_priorities(&session_id).await.unwrap();

        assert_eq!(assessments.len(), 3);
        assert!(assessments.iter().any(|a| a.proposal_id == p1.id));
        assert!(assessments.iter().any(|a| a.proposal_id == p2.id));
        assert!(assessments.iter().any(|a| a.proposal_id == p3.id));
    }

    #[tokio::test]
    async fn test_assess_and_update_all_priorities() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let proposal_repo =
            Arc::new(MockTaskProposalRepository::with_proposals(vec![p1.clone(), p2.clone()]));
        let dep_repo = Arc::new(MockProposalDependencyRepository::new());
        let service = PriorityService::new(proposal_repo.clone(), dep_repo);

        let assessments = service
            .assess_and_update_all_priorities(&session_id)
            .await
            .unwrap();

        assert_eq!(assessments.len(), 2);

        // Verify that update_priority was called for each proposal
        let updated = proposal_repo.get_updated_priorities();
        assert_eq!(updated.len(), 2);
    }

    // ============================================================================
    // Critical Path Tests
    // ============================================================================

    #[tokio::test]
    async fn test_critical_path_on_chain() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "First");
        let p2 = create_test_proposal(&session_id, "Second");
        let p3 = create_test_proposal(&session_id, "Third");

        // Linear chain: p3 -> p2 -> p1
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p2.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        // All should be on critical path
        assert!(graph.is_on_critical_path(&p1.id));
        assert!(graph.is_on_critical_path(&p2.id));
        assert!(graph.is_on_critical_path(&p3.id));

        // p1 is on critical path, so should get critical path factor
        let assessment = service.assess_priority(&p1, &graph).await.unwrap();
        assert!(assessment.factors.critical_path_factor.is_on_critical_path);
        assert!(assessment.factors.critical_path_factor.score > 0);
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_high_priority_proposal() {
        let session_id = IdeationSessionId::new();

        // Create a proposal that should score high:
        // - Blocks multiple tasks
        // - On critical path
        // - Contains critical keywords
        // - Is trivial complexity
        // - Has urgency hints
        let mut blocker = TaskProposal::new(
            session_id.clone(),
            "Critical MVP blocker",
            TaskCategory::Feature,
            Priority::Medium,
        );
        blocker.description = Some("URGENT: This must be done ASAP".to_string());
        blocker.estimated_complexity = Complexity::Trivial;

        let dep1 = create_test_proposal(&session_id, "Dependent 1");
        let dep2 = create_test_proposal(&session_id, "Dependent 2");
        let dep3 = create_test_proposal(&session_id, "Dependent 3");

        // All three depend on blocker
        let deps = vec![
            (dep1.id.clone(), blocker.id.clone()),
            (dep2.id.clone(), blocker.id.clone()),
            (dep3.id.clone(), blocker.id.clone()),
        ];

        let service = create_service(
            vec![blocker.clone(), dep1.clone(), dep2.clone(), dep3.clone()],
            deps,
        );
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let assessment = service.assess_priority(&blocker, &graph).await.unwrap();

        // Should score very high
        assert!(
            assessment.priority_score >= 60,
            "Expected high score, got {}",
            assessment.priority_score
        );
        assert!(
            assessment.suggested_priority == Priority::High
                || assessment.suggested_priority == Priority::Critical
        );
    }

    #[tokio::test]
    async fn test_low_priority_proposal() {
        let session_id = IdeationSessionId::new();

        // Create a proposal that should score low:
        // - Blocks nothing
        // - Not on critical path
        // - Contains low-priority keywords
        // - Is very complex
        // - No urgency hints
        let mut low_prio = TaskProposal::new(
            session_id.clone(),
            "Nice to have feature for later",
            TaskCategory::Feature,
            Priority::Medium,
        );
        low_prio.description = Some("Optional enhancement, not essential".to_string());
        low_prio.estimated_complexity = Complexity::VeryComplex;

        let service = create_service(vec![low_prio.clone()], vec![]);
        let graph = service.build_dependency_graph(&session_id).await.unwrap();

        let assessment = service.assess_priority(&low_prio, &graph).await.unwrap();

        // Should score low
        assert!(
            assessment.priority_score < 40,
            "Expected low score, got {}",
            assessment.priority_score
        );
        assert_eq!(assessment.suggested_priority, Priority::Low);
    }