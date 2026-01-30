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

        for (from, to, _reason) in &dependencies {
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
        // TODO: Task 5 will add reason to DependencyGraphEdge
        let edges: Vec<DependencyGraphEdge> = dependencies
            .iter()
            .map(|(from, to, _reason)| DependencyGraphEdge::new(from.clone(), to.clone()))
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
#[path = "tests.rs"]
mod tests;
