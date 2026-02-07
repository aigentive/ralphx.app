use super::DependencyService;
use crate::domain::entities::{
    ArtifactId, IdeationSessionId, Priority, TaskCategory, TaskProposal, TaskProposalId,
};
use crate::domain::repositories::{ProposalDependencyRepository, TaskProposalRepository};
use crate::error::AppResult;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

    // ============================================================================
    // Mock Repositories
    // ============================================================================

    struct MockTaskProposalRepository {
        proposals: Mutex<Vec<TaskProposal>>,
    }

    impl MockTaskProposalRepository {
        fn with_proposals(proposals: Vec<TaskProposal>) -> Self {
            Self {
                proposals: Mutex::new(proposals),
            }
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
            _id: &TaskProposalId,
            _assessment: &crate::domain::entities::PriorityAssessment,
        ) -> AppResult<()> {
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

        async fn clear_created_task_ids_by_session(
            &self,
            _session_id: &IdeationSessionId,
        ) -> AppResult<()> {
            Ok(())
        }
    }

    struct MockProposalDependencyRepository {
        dependencies: Mutex<HashMap<TaskProposalId, HashSet<TaskProposalId>>>,
    }

    impl MockProposalDependencyRepository {
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
            // Simple check: would adding from->to create a cycle?
            // Check if there's already a path from depends_on_id to proposal_id
            let deps = self.dependencies.lock().unwrap();

            // BFS to check reachability
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(depends_on_id.clone());

            while let Some(current) = queue.pop_front() {
                if &current == proposal_id {
                    return Ok(true); // Found a path back, would create cycle
                }
                if visited.insert(current.clone()) {
                    if let Some(neighbors) = deps.get(&current) {
                        for neighbor in neighbors {
                            queue.push_back(neighbor.clone());
                        }
                    }
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

    fn create_proposal_with_category(
        session_id: &IdeationSessionId,
        title: &str,
        category: TaskCategory,
    ) -> TaskProposal {
        TaskProposal::new(session_id.clone(), title, category, Priority::Medium)
    }

    fn create_service(
        proposals: Vec<TaskProposal>,
        dependencies: Vec<(TaskProposalId, TaskProposalId)>,
    ) -> DependencyService<MockTaskProposalRepository, MockProposalDependencyRepository> {
        let proposal_repo = Arc::new(MockTaskProposalRepository::with_proposals(proposals));
        let dep_repo = Arc::new(MockProposalDependencyRepository::with_dependencies(
            dependencies,
        ));
        DependencyService::new(proposal_repo, dep_repo)
    }

    // ============================================================================
    // Build Graph Tests
    // ============================================================================

    #[tokio::test]
    async fn test_build_graph_empty_session() {
        let service = create_service(vec![], vec![]);
        let session_id = IdeationSessionId::new();

        let graph = service.build_graph(&session_id).await.unwrap();

        assert!(graph.is_empty());
        assert!(!graph.has_cycles);
        assert!(graph.critical_path.is_empty());
    }

    #[tokio::test]
    async fn test_build_graph_single_proposal() {
        let session_id = IdeationSessionId::new();
        let proposal = create_test_proposal(&session_id, "Test task");

        let service = create_service(vec![proposal.clone()], vec![]);

        let graph = service.build_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.has_cycles);
        assert_eq!(graph.critical_path_length(), 1); // Single node is the path
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

        let graph = service.build_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert!(!graph.has_cycles);
        assert_eq!(graph.critical_path_length(), 3);
    }

    #[tokio::test]
    async fn test_build_graph_parallel_tasks() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Root");
        let p2 = create_test_proposal(&session_id, "Branch A");
        let p3 = create_test_proposal(&session_id, "Branch B");

        // Both p2 and p3 depend on p1 (diamond start)
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p1.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let graph = service.build_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert!(!graph.has_cycles);
        // Critical path is 2 (p1 -> p2 or p1 -> p3)
        assert_eq!(graph.critical_path_length(), 2);
    }

    #[tokio::test]
    async fn test_build_graph_diamond_pattern() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Root");
        let p2 = create_test_proposal(&session_id, "Left");
        let p3 = create_test_proposal(&session_id, "Right");
        let p4 = create_test_proposal(&session_id, "End");

        // Diamond: p2,p3 depend on p1; p4 depends on p2,p3
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p1.id.clone()),
            (p4.id.clone(), p2.id.clone()),
            (p4.id.clone(), p3.id.clone()),
        ];

        let service = create_service(
            vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()],
            deps,
        );

        let graph = service.build_graph(&session_id).await.unwrap();

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 4);
        assert!(!graph.has_cycles);
        assert_eq!(graph.critical_path_length(), 3); // p1 -> p2|p3 -> p4
    }

    // ============================================================================
    // Detect Cycles Tests
    // ============================================================================

    #[test]
    fn test_detect_cycles_no_cycles() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let deps = vec![(p2.id.clone(), p1.id.clone())];
        let service = create_service(vec![p1.clone(), p2.clone()], deps.clone());

        let cycles = service.detect_cycles(&[p1, p2], &deps);

        assert!(cycles.is_empty());
    }

    #[test]
    fn test_detect_cycles_simple_cycle() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        // Circular: p1 -> p2 -> p1
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p1.id.clone()),
        ];
        let service = create_service(vec![p1.clone(), p2.clone()], deps.clone());

        let cycles = service.detect_cycles(&[p1, p2], &deps);

        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_detect_cycles_three_node_cycle() {
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
        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps.clone());

        let cycles = service.detect_cycles(&[p1, p2, p3], &deps);

        assert!(!cycles.is_empty());
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

        let graph = service.build_graph(&session_id).await.unwrap();

        assert!(graph.has_cycles);
        assert!(graph.cycles.is_some());
        assert!(graph.critical_path.is_empty()); // Can't compute with cycles
    }

    // ============================================================================
    // Find Critical Path Tests
    // ============================================================================

    #[test]
    fn test_find_critical_path_empty() {
        let service = create_service(vec![], vec![]);
        let path = service.find_critical_path(&[], &[]);

        assert!(path.is_empty());
    }

    #[test]
    fn test_find_critical_path_single_node() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Solo");

        let service = create_service(vec![p1.clone()], vec![]);
        let path = service.find_critical_path(&[p1.clone()], &[]);

        assert_eq!(path.len(), 1);
        assert_eq!(path[0], p1.id);
    }

    #[test]
    fn test_find_critical_path_linear_chain() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "First");
        let p2 = create_test_proposal(&session_id, "Second");
        let p3 = create_test_proposal(&session_id, "Third");

        // Linear: p2 depends on p1, p3 depends on p2
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p2.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps.clone());
        let path = service.find_critical_path(&[p1.clone(), p2.clone(), p3.clone()], &deps);

        assert_eq!(path.len(), 3);
        // Path should be p1 -> p2 -> p3
        assert_eq!(path[0], p1.id);
        assert_eq!(path[1], p2.id);
        assert_eq!(path[2], p3.id);
    }

    #[test]
    fn test_find_critical_path_with_branches() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Root");
        let p2 = create_test_proposal(&session_id, "Short");
        let p3 = create_test_proposal(&session_id, "Long1");
        let p4 = create_test_proposal(&session_id, "Long2");

        // p2 depends on p1 (short branch)
        // p3 depends on p1, p4 depends on p3 (long branch)
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p1.id.clone()),
            (p4.id.clone(), p3.id.clone()),
        ];

        let service = create_service(
            vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()],
            deps.clone(),
        );
        let path = service.find_critical_path(
            &[p1.clone(), p2.clone(), p3.clone(), p4.clone()],
            &deps,
        );

        // Critical path should be the longer one: p1 -> p3 -> p4
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], p1.id);
        assert_eq!(path[1], p3.id);
        assert_eq!(path[2], p4.id);
    }

    #[test]
    fn test_find_critical_path_returns_empty_on_cycle() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        // Circular
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p1.id.clone()),
        ];

        let service = create_service(vec![p1.clone(), p2.clone()], deps.clone());
        let path = service.find_critical_path(&[p1, p2], &deps);

        assert!(path.is_empty());
    }

    // ============================================================================
    // Suggest Dependencies Tests
    // ============================================================================

    #[test]
    fn test_suggest_dependencies_empty() {
        let service = create_service(vec![], vec![]);
        let suggestions = service.suggest_dependencies(&[]);

        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggest_dependencies_setup_before_feature() {
        let session_id = IdeationSessionId::new();
        let setup = create_proposal_with_category(&session_id, "Setup database", TaskCategory::Setup);
        let feature = create_proposal_with_category(&session_id, "Add user auth", TaskCategory::Feature);

        let service = create_service(vec![setup.clone(), feature.clone()], vec![]);
        let suggestions = service.suggest_dependencies(&[setup.clone(), feature.clone()]);

        // Should suggest that feature depends on setup
        assert!(!suggestions.is_empty());
        let has_setup_dep = suggestions
            .iter()
            .any(|(from, to)| from == &feature.id && to == &setup.id);
        assert!(has_setup_dep, "Feature should depend on setup");
    }

    #[test]
    fn test_suggest_dependencies_test_after_feature() {
        let session_id = IdeationSessionId::new();
        let feature = create_proposal_with_category(&session_id, "Add user auth", TaskCategory::Feature);
        let test = create_proposal_with_category(&session_id, "Test user auth", TaskCategory::Test);

        let service = create_service(vec![feature.clone(), test.clone()], vec![]);
        let suggestions = service.suggest_dependencies(&[feature.clone(), test.clone()]);

        // Should suggest that test depends on feature
        let has_test_dep = suggestions
            .iter()
            .any(|(from, to)| from == &test.id && to == &feature.id);
        assert!(has_test_dep, "Test should depend on feature");
    }

    // ============================================================================
    // Validate No Cycles Tests
    // ============================================================================

    #[tokio::test]
    async fn test_validate_no_cycles_empty() {
        let service = create_service(vec![], vec![]);
        let result = service.validate_no_cycles(&[]).await.unwrap();

        assert!(result.is_valid);
        assert!(result.cycles.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_validate_no_cycles_valid_selection() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let deps = vec![(p2.id.clone(), p1.id.clone())];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        let result = service
            .validate_no_cycles(&[p1.id.clone(), p2.id.clone()])
            .await
            .unwrap();

        assert!(result.is_valid);
        assert!(result.cycles.is_empty());
    }

    #[tokio::test]
    async fn test_validate_no_cycles_invalid_selection() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        // Circular dependency
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p1.id.clone()),
        ];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        let result = service
            .validate_no_cycles(&[p1.id.clone(), p2.id.clone()])
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.cycles.is_empty());
        assert!(!result.warnings.is_empty());
    }

    // ============================================================================
    // Validate Dependency Tests
    // ============================================================================

    #[tokio::test]
    async fn test_validate_dependency_self_reference() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");

        let service = create_service(vec![p1.clone()], vec![]);

        // Self-dependency should be invalid
        let is_valid = service
            .validate_dependency(&session_id, &p1.id, &p1.id)
            .await
            .unwrap();

        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_validate_dependency_would_create_cycle() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        // p1 depends on p2
        let deps = vec![(p1.id.clone(), p2.id.clone())];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        // Adding p2 depends on p1 would create cycle
        let is_valid = service
            .validate_dependency(&session_id, &p2.id, &p1.id)
            .await
            .unwrap();

        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_validate_dependency_valid() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let service = create_service(vec![p1.clone(), p2.clone()], vec![]);

        // Adding p2 depends on p1 is valid
        let is_valid = service
            .validate_dependency(&session_id, &p2.id, &p1.id)
            .await
            .unwrap();

        assert!(is_valid);
    }

    // ============================================================================
    // Analyze Dependencies Tests
    // ============================================================================

    #[tokio::test]
    async fn test_analyze_dependencies_empty() {
        let service = create_service(vec![], vec![]);
        let session_id = IdeationSessionId::new();

        let analysis = service.analyze_dependencies(&session_id).await.unwrap();

        assert!(analysis.graph.is_empty());
        assert!(analysis.roots.is_empty());
        assert!(analysis.leaves.is_empty());
        assert!(analysis.blockers.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_dependencies_identifies_roots() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Root");
        let p2 = create_test_proposal(&session_id, "Child");

        let deps = vec![(p2.id.clone(), p1.id.clone())];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        let analysis = service.analyze_dependencies(&session_id).await.unwrap();

        assert!(analysis.roots.contains(&p1.id));
        assert!(!analysis.roots.contains(&p2.id));
    }

    #[tokio::test]
    async fn test_analyze_dependencies_identifies_leaves() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Root");
        let p2 = create_test_proposal(&session_id, "Leaf");

        let deps = vec![(p2.id.clone(), p1.id.clone())];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        let analysis = service.analyze_dependencies(&session_id).await.unwrap();

        assert!(analysis.leaves.contains(&p2.id));
        assert!(!analysis.leaves.contains(&p1.id));
    }

    #[tokio::test]
    async fn test_analyze_dependencies_identifies_blockers() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "Blocker");
        let p2 = create_test_proposal(&session_id, "Blocked 1");
        let p3 = create_test_proposal(&session_id, "Blocked 2");

        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p1.id.clone()),
        ];
        let service = create_service(vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let analysis = service.analyze_dependencies(&session_id).await.unwrap();

        assert!(analysis.blockers.contains(&p1.id));
        assert!(!analysis.blockers.contains(&p2.id));
        assert!(!analysis.blockers.contains(&p3.id));
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_full_workflow_build_and_analyze() {
        let session_id = IdeationSessionId::new();

        // Create a realistic project structure
        let setup = create_proposal_with_category(&session_id, "Setup project", TaskCategory::Setup);
        let feature1 = create_proposal_with_category(&session_id, "User auth", TaskCategory::Feature);
        let feature2 = create_proposal_with_category(&session_id, "User profile", TaskCategory::Feature);
        let test = create_proposal_with_category(&session_id, "Integration tests", TaskCategory::Test);

        // Dependencies: feature1 depends on setup, feature2 depends on feature1, test depends on both features
        let deps = vec![
            (feature1.id.clone(), setup.id.clone()),
            (feature2.id.clone(), feature1.id.clone()),
            (test.id.clone(), feature1.id.clone()),
            (test.id.clone(), feature2.id.clone()),
        ];

        let service = create_service(
            vec![setup.clone(), feature1.clone(), feature2.clone(), test.clone()],
            deps,
        );

        let analysis = service.analyze_dependencies(&session_id).await.unwrap();

        // Verify structure
        assert_eq!(analysis.graph.node_count(), 4);
        assert_eq!(analysis.graph.edge_count(), 4);
        assert!(!analysis.graph.has_cycles);

        // Setup is the only root
        assert_eq!(analysis.roots.len(), 1);
        assert!(analysis.roots.contains(&setup.id));

        // Test is the only leaf
        assert_eq!(analysis.leaves.len(), 1);
        assert!(analysis.leaves.contains(&test.id));

        // Setup, feature1, and feature2 are blockers
        assert!(analysis.blockers.contains(&setup.id));
        assert!(analysis.blockers.contains(&feature1.id));
        assert!(analysis.blockers.contains(&feature2.id));
        assert!(!analysis.blockers.contains(&test.id));
    }

    #[tokio::test]
    async fn test_validation_result_formatting() {
        let session_id = IdeationSessionId::new();
        let p1 = create_test_proposal(&session_id, "A");
        let p2 = create_test_proposal(&session_id, "B");

        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p1.id.clone()),
        ];
        let service = create_service(vec![p1.clone(), p2.clone()], deps);

        let result = service
            .validate_no_cycles(&[p1.id.clone(), p2.id.clone()])
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("circular dependency"));
    }
