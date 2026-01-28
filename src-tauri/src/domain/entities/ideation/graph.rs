//! Dependency graph structures

use serde::{Deserialize, Serialize};

use crate::domain::entities::TaskProposalId;

/// A node in the dependency graph representing a single proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraphNode {
    /// The proposal ID this node represents
    pub proposal_id: TaskProposalId,
    /// Title of the proposal for display
    pub title: String,
    /// Number of dependencies (proposals this depends on)
    pub in_degree: usize,
    /// Number of dependents (proposals that depend on this)
    pub out_degree: usize,
}

impl DependencyGraphNode {
    /// Create a new dependency graph node
    pub fn new(proposal_id: TaskProposalId, title: impl Into<String>) -> Self {
        Self {
            proposal_id,
            title: title.into(),
            in_degree: 0,
            out_degree: 0,
        }
    }

    /// Set the in-degree (dependency count)
    pub fn with_in_degree(mut self, count: usize) -> Self {
        self.in_degree = count;
        self
    }

    /// Set the out-degree (dependent count)
    pub fn with_out_degree(mut self, count: usize) -> Self {
        self.out_degree = count;
        self
    }

    /// Returns true if this node has no dependencies (is a root)
    pub fn is_root(&self) -> bool {
        self.in_degree == 0
    }

    /// Returns true if this node has no dependents (is a leaf)
    pub fn is_leaf(&self) -> bool {
        self.out_degree == 0
    }

    /// Returns true if this node is a blocker (has dependents)
    pub fn is_blocker(&self) -> bool {
        self.out_degree > 0
    }
}

/// An edge in the dependency graph representing a dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraphEdge {
    /// The proposal that has a dependency (depends on "to")
    pub from: TaskProposalId,
    /// The proposal that is depended on (is a dependency of "from")
    pub to: TaskProposalId,
}

impl DependencyGraphEdge {
    /// Create a new dependency edge
    /// "from" depends on "to" (from → to means from needs to complete first)
    pub fn new(from: TaskProposalId, to: TaskProposalId) -> Self {
        Self { from, to }
    }
}

/// Complete dependency graph for proposals in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All nodes in the graph
    pub nodes: Vec<DependencyGraphNode>,
    /// All edges in the graph
    pub edges: Vec<DependencyGraphEdge>,
    /// The critical path (longest path through the graph)
    pub critical_path: Vec<TaskProposalId>,
    /// Whether the graph contains any cycles
    pub has_cycles: bool,
    /// If cycles exist, the proposals involved in each cycle
    pub cycles: Option<Vec<Vec<TaskProposalId>>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            critical_path: Vec::new(),
            has_cycles: false,
            cycles: None,
        }
    }

    /// Create a dependency graph with nodes and edges
    pub fn with_nodes_and_edges(
        nodes: Vec<DependencyGraphNode>,
        edges: Vec<DependencyGraphEdge>,
    ) -> Self {
        Self {
            nodes,
            edges,
            critical_path: Vec::new(),
            has_cycles: false,
            cycles: None,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: DependencyGraphNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: DependencyGraphEdge) {
        self.edges.push(edge);
    }

    /// Set the critical path
    pub fn set_critical_path(&mut self, path: Vec<TaskProposalId>) {
        self.critical_path = path;
    }

    /// Mark the graph as having cycles and record them
    pub fn set_cycles(&mut self, cycles: Vec<Vec<TaskProposalId>>) {
        self.has_cycles = !cycles.is_empty();
        self.cycles = if cycles.is_empty() {
            None
        } else {
            Some(cycles)
        };
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get a node by proposal ID
    pub fn get_node(&self, proposal_id: &TaskProposalId) -> Option<&DependencyGraphNode> {
        self.nodes.iter().find(|n| n.proposal_id == *proposal_id)
    }

    /// Get all edges where the given proposal is the source (depends on others)
    pub fn get_dependencies(&self, proposal_id: &TaskProposalId) -> Vec<&DependencyGraphEdge> {
        self.edges.iter().filter(|e| e.from == *proposal_id).collect()
    }

    /// Get all edges where the given proposal is the target (is depended on)
    pub fn get_dependents(&self, proposal_id: &TaskProposalId) -> Vec<&DependencyGraphEdge> {
        self.edges.iter().filter(|e| e.to == *proposal_id).collect()
    }

    /// Get all root nodes (nodes with no dependencies)
    pub fn get_roots(&self) -> Vec<&DependencyGraphNode> {
        self.nodes.iter().filter(|n| n.is_root()).collect()
    }

    /// Get all leaf nodes (nodes with no dependents)
    pub fn get_leaves(&self) -> Vec<&DependencyGraphNode> {
        self.nodes.iter().filter(|n| n.is_leaf()).collect()
    }

    /// Check if a proposal is on the critical path
    pub fn is_on_critical_path(&self, proposal_id: &TaskProposalId) -> bool {
        self.critical_path.contains(proposal_id)
    }

    /// Get the length of the critical path
    pub fn critical_path_length(&self) -> usize {
        self.critical_path.len()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

