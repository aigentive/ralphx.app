// DependencyService
// Application service for analyzing task proposal dependencies
//
// This service provides:
// - Building dependency graphs from proposals
// - Cycle detection using DFS
// - Critical path calculation using topological sort + longest path
// - Dependency suggestion (stub for AI-based inference)
// - Validation for apply workflow (no cycles in selection)

#[cfg(test)]
mod tests;

mod types;

pub use types::{DependencyAnalysis, ValidationResult};

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
