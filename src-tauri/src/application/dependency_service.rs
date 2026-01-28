// DependencyService
// Application service for analyzing task proposal dependencies
//
// This service provides:
// - Building dependency graphs from proposals
// - Cycle detection using DFS
// - Critical path calculation using topological sort + longest path
// - Dependency suggestion (stub for AI-based inference)
// - Validation for apply workflow (no cycles in selection)

use crate::domain::entities::{
    DependencyGraph, DependencyGraphEdge, DependencyGraphNode, IdeationSessionId, TaskProposal,
    TaskProposalId,
};
use crate::domain::repositories::{ProposalDependencyRepository, TaskProposalRepository};
use crate::error::AppResult;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Service for analyzing task proposal dependencies
pub struct DependencyService<P: TaskProposalRepository, D: ProposalDependencyRepository> {
    /// Repository for task proposals
    proposal_repo: Arc<P>,
    /// Repository for proposal dependencies
    dependency_repo: Arc<D>,
}

impl<P: TaskProposalRepository, D: ProposalDependencyRepository> DependencyService<P, D> {
    /// Create a new dependency service
    pub fn new(proposal_repo: Arc<P>, dependency_repo: Arc<D>) -> Self {
        Self {
            proposal_repo,
            dependency_repo,
        }
    }

    /// Build a dependency graph for all proposals in a session
    ///
    /// This method:
    /// 1. Fetches all proposals for the session
    /// 2. Fetches all dependencies between proposals
    /// 3. Builds adjacency lists for the graph
    /// 4. Creates nodes with in/out degree counts
    /// 5. Detects cycles using DFS
    /// 6. Computes the critical path (if no cycles)
    pub async fn build_graph(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<DependencyGraph> {
        // Get all proposals for the session
        let proposals = self.proposal_repo.get_by_session(session_id).await?;

        // Get all dependencies for the session
        let dependencies = self.dependency_repo.get_all_for_session(session_id).await?;

        // Build the graph from proposals and dependencies
        self.build_graph_from_data(&proposals, &dependencies)
    }

    /// Build a dependency graph from provided proposals and dependencies
    /// (useful for testing and when data is already available)
    pub fn build_graph_from_data(
        &self,
        proposals: &[TaskProposal],
        dependencies: &[(TaskProposalId, TaskProposalId)],
    ) -> AppResult<DependencyGraph> {
        // Build adjacency lists
        // from_map: proposal_id -> list of proposals it depends on
        // to_map: proposal_id -> list of proposals that depend on it (dependents)
        let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

        for (from, to) in dependencies {
            from_map.entry(from.clone()).or_default().push(to.clone());
            to_map.entry(to.clone()).or_default().push(from.clone());
        }

        // Build nodes with degree counts
        let mut nodes = Vec::new();
        for proposal in proposals {
            let in_degree = from_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
            let out_degree = to_map.get(&proposal.id).map(|v| v.len()).unwrap_or(0);
            let node = DependencyGraphNode::new(proposal.id.clone(), &proposal.title)
                .with_in_degree(in_degree)
                .with_out_degree(out_degree);
            nodes.push(node);
        }

        // Build edges
        let edges: Vec<DependencyGraphEdge> = dependencies
            .iter()
            .map(|(from, to)| DependencyGraphEdge::new(from.clone(), to.clone()))
            .collect();

        // Detect cycles using DFS
        let cycles = self.detect_cycles_internal(proposals, &from_map);

        // Find critical path (longest path through the DAG)
        let critical_path = if cycles.is_empty() {
            self.find_critical_path_internal(proposals, &from_map)
        } else {
            Vec::new() // Can't compute critical path with cycles
        };

        let mut graph = DependencyGraph::with_nodes_and_edges(nodes, edges);
        graph.set_critical_path(critical_path);
        graph.set_cycles(cycles);

        Ok(graph)
    }

    /// Detect cycles in the dependency graph using DFS
    ///
    /// Returns a list of cycles, where each cycle is a list of proposal IDs
    /// in the order they form the cycle.
    pub fn detect_cycles(
        &self,
        proposals: &[TaskProposal],
        dependencies: &[(TaskProposalId, TaskProposalId)],
    ) -> Vec<Vec<TaskProposalId>> {
        // Build adjacency list
        let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        for (from, to) in dependencies {
            from_map.entry(from.clone()).or_default().push(to.clone());
        }

        self.detect_cycles_internal(proposals, &from_map)
    }

    /// Internal cycle detection using DFS
    fn detect_cycles_internal(
        &self,
        proposals: &[TaskProposal],
        from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    ) -> Vec<Vec<TaskProposalId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for proposal in proposals {
            if !visited.contains(&proposal.id) {
                Self::dfs_detect_cycle(
                    &proposal.id,
                    from_map,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycle(
        node: &TaskProposalId,
        from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
        visited: &mut HashSet<TaskProposalId>,
        rec_stack: &mut HashSet<TaskProposalId>,
        path: &mut Vec<TaskProposalId>,
        cycles: &mut Vec<Vec<TaskProposalId>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(neighbors) = from_map.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    Self::dfs_detect_cycle(neighbor, from_map, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle - extract it from the path
                    if let Some(start_idx) = path.iter().position(|n| n == neighbor) {
                        let cycle: Vec<TaskProposalId> = path[start_idx..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Find the critical path (longest path) in the DAG
    ///
    /// Uses Kahn's algorithm for topological sort followed by
    /// dynamic programming to find the longest path.
    ///
    /// Returns empty if the graph has cycles.
    pub fn find_critical_path(
        &self,
        proposals: &[TaskProposal],
        dependencies: &[(TaskProposalId, TaskProposalId)],
    ) -> Vec<TaskProposalId> {
        // Check for cycles first
        let cycles = self.detect_cycles(proposals, dependencies);
        if !cycles.is_empty() {
            return Vec::new();
        }

        // Build adjacency list
        let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        for (from, to) in dependencies {
            from_map.entry(from.clone()).or_default().push(to.clone());
        }

        self.find_critical_path_internal(proposals, &from_map)
    }

    /// Internal critical path calculation
    fn find_critical_path_internal(
        &self,
        proposals: &[TaskProposal],
        from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    ) -> Vec<TaskProposalId> {
        if proposals.is_empty() {
            return Vec::new();
        }

        // Build reverse map (to_map) for topological sort
        // to_map: proposal_id -> list of proposals that depend on this (get unblocked when this completes)
        let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        let mut in_degree: HashMap<TaskProposalId, usize> = HashMap::new();

        // Initialize all nodes with zero in-degree
        for proposal in proposals {
            in_degree.insert(proposal.id.clone(), 0);
        }

        // Build reverse adjacency and count in-degrees
        for (from, deps) in from_map {
            for to in deps {
                to_map.entry(to.clone()).or_default().push(from.clone());
                *in_degree.entry(from.clone()).or_default() += 1;
            }
        }

        // Topological sort using Kahn's algorithm
        let mut queue: VecDeque<TaskProposalId> = VecDeque::new();
        for (id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut topo_order = Vec::new();
        while let Some(node) = queue.pop_front() {
            topo_order.push(node.clone());

            if let Some(neighbors) = to_map.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // If we couldn't process all nodes, there's a cycle
        if topo_order.len() != proposals.len() {
            return Vec::new();
        }

        // DP to find longest path
        let mut dist: HashMap<TaskProposalId, i32> = HashMap::new();
        let mut prev: HashMap<TaskProposalId, Option<TaskProposalId>> = HashMap::new();

        for id in &topo_order {
            dist.insert(id.clone(), 0);
            prev.insert(id.clone(), None);
        }

        // Process nodes in topological order
        for node in &topo_order {
            if let Some(neighbors) = to_map.get(node) {
                for neighbor in neighbors {
                    let new_dist = dist.get(node).unwrap_or(&0) + 1;
                    if new_dist > *dist.get(neighbor).unwrap_or(&0) {
                        dist.insert(neighbor.clone(), new_dist);
                        prev.insert(neighbor.clone(), Some(node.clone()));
                    }
                }
            }
        }

        // Find the node with maximum distance (end of critical path)
        let mut max_dist = 0;
        let mut end_node: Option<TaskProposalId> = topo_order.first().cloned();

        for (id, &d) in &dist {
            if d > max_dist {
                max_dist = d;
                end_node = Some(id.clone());
            }
        }

        // Reconstruct the path from end to start
        let mut path = Vec::new();
        let mut current = end_node;

        while let Some(node) = current {
            path.push(node.clone());
            current = prev.get(&node).and_then(|p| p.clone());
        }

        path.reverse();
        path
    }

    /// Suggest dependencies between proposals based on their content
    ///
    /// This is a stub implementation that returns empty suggestions.
    /// In a full implementation, this could use:
    /// - Keyword analysis (setup before feature, tests after implementation)
    /// - Natural language processing
    /// - AI-based inference
    pub fn suggest_dependencies(
        &self,
        proposals: &[TaskProposal],
    ) -> Vec<(TaskProposalId, TaskProposalId)> {
        // Stub implementation - simple heuristic based on categories
        // In a real implementation, this would use more sophisticated analysis
        let mut suggestions = Vec::new();

        // Find setup tasks
        let setup_tasks: Vec<&TaskProposal> = proposals
            .iter()
            .filter(|p| p.category == crate::domain::entities::TaskCategory::Setup)
            .collect();

        // Find non-setup tasks
        let other_tasks: Vec<&TaskProposal> = proposals
            .iter()
            .filter(|p| p.category != crate::domain::entities::TaskCategory::Setup)
            .collect();

        // Suggest that non-setup tasks depend on setup tasks
        for setup in &setup_tasks {
            for other in &other_tasks {
                // Don't suggest if they're the same task
                if setup.id != other.id {
                    suggestions.push((other.id.clone(), setup.id.clone()));
                }
            }
        }

        // Find testing tasks
        let testing_tasks: Vec<&TaskProposal> = proposals
            .iter()
            .filter(|p| p.category == crate::domain::entities::TaskCategory::Test)
            .collect();

        // Find feature tasks
        let feature_tasks: Vec<&TaskProposal> = proposals
            .iter()
            .filter(|p| p.category == crate::domain::entities::TaskCategory::Feature)
            .collect();

        // Suggest that testing tasks depend on feature tasks
        for test in &testing_tasks {
            for feature in &feature_tasks {
                if test.id != feature.id {
                    suggestions.push((test.id.clone(), feature.id.clone()));
                }
            }
        }

        suggestions
    }

    /// Validate that a selection of proposals has no cycles
    ///
    /// Used before applying proposals to Kanban to ensure
    /// the resulting task dependencies are valid.
    pub async fn validate_no_cycles(
        &self,
        proposal_ids: &[TaskProposalId],
    ) -> AppResult<ValidationResult> {
        if proposal_ids.is_empty() {
            return Ok(ValidationResult::valid());
        }

        // Get proposals
        let mut proposals = Vec::new();
        for id in proposal_ids {
            if let Some(proposal) = self.proposal_repo.get_by_id(id).await? {
                proposals.push(proposal);
            }
        }

        // Get dependencies only between selected proposals
        let selected_set: HashSet<&TaskProposalId> = proposal_ids.iter().collect();
        let mut relevant_deps = Vec::new();

        for id in proposal_ids {
            let deps = self.dependency_repo.get_dependencies(id).await?;
            for dep_id in deps {
                if selected_set.contains(&dep_id) {
                    relevant_deps.push((id.clone(), dep_id));
                }
            }
        }

        // Detect cycles
        let cycles = self.detect_cycles(&proposals, &relevant_deps);

        if cycles.is_empty() {
            Ok(ValidationResult::valid())
        } else {
            Ok(ValidationResult::invalid_with_cycles(cycles))
        }
    }

    /// Validate that adding a specific dependency would not create a cycle
    pub async fn validate_dependency(
        &self,
        _session_id: &IdeationSessionId,
        from_id: &TaskProposalId,
        to_id: &TaskProposalId,
    ) -> AppResult<bool> {
        // Self-dependency is always invalid
        if from_id == to_id {
            return Ok(false);
        }

        // Check if this would create a cycle using the repository method
        self.dependency_repo.would_create_cycle(from_id, to_id).await
            .map(|would_cycle| !would_cycle)
    }

    /// Get the dependency graph for visualization
    pub async fn analyze_dependencies(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<DependencyAnalysis> {
        let graph = self.build_graph(session_id).await?;

        let roots: Vec<TaskProposalId> = graph
            .get_roots()
            .iter()
            .map(|n| n.proposal_id.clone())
            .collect();

        let leaves: Vec<TaskProposalId> = graph
            .get_leaves()
            .iter()
            .map(|n| n.proposal_id.clone())
            .collect();

        let blockers: Vec<TaskProposalId> = graph
            .nodes
            .iter()
            .filter(|n| n.is_blocker())
            .map(|n| n.proposal_id.clone())
            .collect();

        Ok(DependencyAnalysis {
            graph,
            roots,
            leaves,
            blockers,
        })
    }
}

/// Result of validating a proposal selection for cycles
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the selection is valid (no cycles)
    pub is_valid: bool,
    /// Cycles found in the selection (if any)
    pub cycles: Vec<Vec<TaskProposalId>>,
    /// Human-readable warning messages
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            cycles: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with cycles
    pub fn invalid_with_cycles(cycles: Vec<Vec<TaskProposalId>>) -> Self {
        let warnings = cycles
            .iter()
            .enumerate()
            .map(|(i, cycle)| {
                format!(
                    "Cycle {}: {} proposals form a circular dependency",
                    i + 1,
                    cycle.len()
                )
            })
            .collect();

        Self {
            is_valid: false,
            cycles,
            warnings,
        }
    }
}

/// Complete dependency analysis for a session
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// The full dependency graph
    pub graph: DependencyGraph,
    /// Root nodes (no dependencies)
    pub roots: Vec<TaskProposalId>,
    /// Leaf nodes (no dependents)
    pub leaves: Vec<TaskProposalId>,
    /// Blocker nodes (have dependents)
    pub blockers: Vec<TaskProposalId>,
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::domain::entities::{ArtifactId, Priority, TaskCategory};
    use async_trait::async_trait;
    use std::sync::Mutex;

    // ============================================================================
    // Mock Repositories
    // ============================================================================

    struct MockTaskProposalRepository {
        proposals: Mutex<Vec<TaskProposal>>,
    }

    impl MockTaskProposalRepository {
        fn new() -> Self {
            Self {
                proposals: Mutex::new(Vec::new()),
            }
        }

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
    }

    struct MockProposalDependencyRepository {
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
        ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .flat_map(|(from, tos)| tos.iter().map(|to| (from.clone(), to.clone())))
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
}
