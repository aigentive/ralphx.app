// PriorityService
// Application service for calculating and assessing task proposal priorities
//
// This service orchestrates priority calculation using the 5-factor system:
// - Dependency factor (0-30 points): Tasks that unblock others
// - Critical path factor (0-25 points): Tasks on the longest path
// - Business value factor (0-20 points): Keyword-based importance
// - Complexity factor (0-15 points): Simpler tasks score higher
// - User hint factor (0-10 points): Explicit urgency signals

use crate::domain::entities::{
    BusinessValueFactor, Complexity, ComplexityFactor, CriticalPathFactor, DependencyFactor,
    DependencyGraph, DependencyGraphEdge, DependencyGraphNode, IdeationSessionId, Priority,
    PriorityAssessment, PriorityAssessmentFactors, TaskProposal, TaskProposalId, UserHintFactor,
};
use crate::domain::repositories::{ProposalDependencyRepository, TaskProposalRepository};
use crate::error::AppResult;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Service for calculating task proposal priorities
pub struct PriorityService<P: TaskProposalRepository, D: ProposalDependencyRepository> {
    /// Repository for task proposals
    proposal_repo: Arc<P>,
    /// Repository for proposal dependencies
    dependency_repo: Arc<D>,
}

impl<P: TaskProposalRepository, D: ProposalDependencyRepository> PriorityService<P, D> {
    /// Create a new priority service
    pub fn new(proposal_repo: Arc<P>, dependency_repo: Arc<D>) -> Self {
        Self {
            proposal_repo,
            dependency_repo,
        }
    }

    /// Calculate the dependency factor for a proposal based on how many tasks it blocks
    pub fn calculate_dependency_factor(&self, blocks_count: i32) -> DependencyFactor {
        DependencyFactor::calculate(blocks_count)
    }

    /// Calculate the critical path factor for a proposal
    pub fn calculate_critical_path_factor(
        &self,
        is_on_critical_path: bool,
        path_length: i32,
    ) -> CriticalPathFactor {
        CriticalPathFactor::calculate(is_on_critical_path, path_length)
    }

    /// Calculate the business value factor from proposal text
    pub fn calculate_business_value_factor(&self, text: &str) -> BusinessValueFactor {
        BusinessValueFactor::calculate(text)
    }

    /// Calculate the complexity factor (simpler tasks score higher)
    pub fn calculate_complexity_factor(&self, complexity: Complexity) -> ComplexityFactor {
        ComplexityFactor::calculate(complexity)
    }

    /// Calculate the user hint factor from text
    pub fn calculate_user_hint_factor(&self, text: &str) -> UserHintFactor {
        UserHintFactor::calculate(text)
    }

    /// Convert a priority score (0-100) to a Priority level
    pub fn score_to_priority(&self, score: i32) -> Priority {
        PriorityAssessment::score_to_priority(score)
    }

    /// Build a dependency graph for all proposals in a session
    pub async fn build_dependency_graph(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<DependencyGraph> {
        // Get all proposals for the session
        let proposals = self.proposal_repo.get_by_session(session_id).await?;

        // Get all dependencies for the session
        let dependencies = self.dependency_repo.get_all_for_session(session_id).await?;

        // Build adjacency lists
        // from_map: proposal_id -> list of proposals it depends on
        // to_map: proposal_id -> list of proposals that depend on it
        let mut from_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();

        for (from, to) in &dependencies {
            from_map.entry(from.clone()).or_default().push(to.clone());
            to_map.entry(to.clone()).or_default().push(from.clone());
        }

        // Build nodes with degree counts
        let mut nodes = Vec::new();
        for proposal in &proposals {
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
        let cycles = self.detect_cycles(&proposals, &from_map);

        // Find critical path (longest path through the DAG)
        let critical_path = if cycles.is_empty() {
            self.find_critical_path(&proposals, &from_map)
        } else {
            Vec::new() // Can't compute critical path with cycles
        };

        let mut graph = DependencyGraph::with_nodes_and_edges(nodes, edges);
        graph.set_critical_path(critical_path);
        graph.set_cycles(cycles);

        Ok(graph)
    }

    /// Detect cycles in the dependency graph using DFS
    fn detect_cycles(
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

    /// Find the critical path (longest path) in the DAG using topological sort + DP
    fn find_critical_path(
        &self,
        proposals: &[TaskProposal],
        from_map: &HashMap<TaskProposalId, Vec<TaskProposalId>>,
    ) -> Vec<TaskProposalId> {
        if proposals.is_empty() {
            return Vec::new();
        }

        // Build reverse map (to_map) for topological sort
        let mut to_map: HashMap<TaskProposalId, Vec<TaskProposalId>> = HashMap::new();
        let mut in_degree: HashMap<TaskProposalId, usize> = HashMap::new();

        // Initialize
        for proposal in proposals {
            in_degree.insert(proposal.id.clone(), 0);
        }

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

        // If we couldn't process all nodes, there's a cycle (shouldn't happen here)
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

        // Reconstruct the path
        let mut path = Vec::new();
        let mut current = end_node;

        while let Some(node) = current {
            path.push(node.clone());
            current = prev.get(&node).and_then(|p| p.clone());
        }

        path.reverse();
        path
    }

    /// Assess priority for a single proposal
    pub async fn assess_priority(
        &self,
        proposal: &TaskProposal,
        graph: &DependencyGraph,
    ) -> AppResult<PriorityAssessment> {
        // Get the number of dependents (tasks that this proposal blocks)
        let blocks_count = self
            .dependency_repo
            .count_dependents(&proposal.id)
            .await? as i32;

        // Calculate dependency factor
        let dependency_factor = self.calculate_dependency_factor(blocks_count);

        // Calculate critical path factor
        let is_on_critical_path = graph.is_on_critical_path(&proposal.id);
        let path_length = if is_on_critical_path {
            graph.critical_path_length() as i32
        } else {
            0
        };
        let critical_path_factor = self.calculate_critical_path_factor(is_on_critical_path, path_length);

        // Calculate business value factor from title and description
        let text = format!(
            "{} {}",
            proposal.title,
            proposal.description.as_deref().unwrap_or("")
        );
        let business_value_factor = self.calculate_business_value_factor(&text);

        // Calculate complexity factor
        let complexity_factor = self.calculate_complexity_factor(proposal.estimated_complexity);

        // Calculate user hint factor from title and description
        let user_hint_factor = self.calculate_user_hint_factor(&text);

        // Build factors container
        let factors = PriorityAssessmentFactors {
            dependency_factor,
            critical_path_factor,
            business_value_factor,
            complexity_factor,
            user_hint_factor,
        };

        // Create assessment (this calculates total score and suggested priority)
        let assessment = PriorityAssessment::new(proposal.id.clone(), factors);

        Ok(assessment)
    }

    /// Assess priorities for all proposals in a session
    pub async fn assess_all_priorities(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<PriorityAssessment>> {
        // Build the dependency graph once
        let graph = self.build_dependency_graph(session_id).await?;

        // Get all proposals
        let proposals = self.proposal_repo.get_by_session(session_id).await?;

        // Assess each proposal
        let mut assessments = Vec::with_capacity(proposals.len());
        for proposal in &proposals {
            let assessment = self.assess_priority(proposal, &graph).await?;
            assessments.push(assessment);
        }

        Ok(assessments)
    }

    /// Assess and persist priorities for all proposals in a session
    pub async fn assess_and_update_all_priorities(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<PriorityAssessment>> {
        let assessments = self.assess_all_priorities(session_id).await?;

        // Update each proposal with its new priority
        for assessment in &assessments {
            self.proposal_repo
                .update_priority(&assessment.proposal_id, assessment)
                .await?;
        }

        Ok(assessments)
    }
}

#[cfg(test)]
mod tests {
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
        fn new() -> Self {
            Self {
                proposals: Mutex::new(Vec::new()),
                updated_priorities: Mutex::new(Vec::new()),
            }
        }

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
}
