/**
 * DependencyVisualization component tests
 *
 * Tests for:
 * - Graph rendering with nodes and edges
 * - Lines connecting dependent proposals (SVG)
 * - Critical path highlighting
 * - Cycle warning indicators
 * - Compact mode for ApplyModal
 * - Empty state
 * - Accessibility
 * - Styling
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import { DependencyVisualization } from "./DependencyVisualization";
import type { DependencyGraph, DependencyGraphNode, DependencyGraphEdge } from "@/types/ideation";

// ============================================================================
// Test Data
// ============================================================================

const createMockNode = (overrides: Partial<DependencyGraphNode> = {}): DependencyGraphNode => ({
  proposalId: "proposal-1",
  title: "Test Proposal",
  inDegree: 0,
  outDegree: 0,
  ...overrides,
});

const createMockEdge = (overrides: Partial<DependencyGraphEdge> = {}): DependencyGraphEdge => ({
  from: "proposal-1",
  to: "proposal-2",
  ...overrides,
});

const createMockGraph = (overrides: Partial<DependencyGraph> = {}): DependencyGraph => ({
  nodes: [
    createMockNode({ proposalId: "p1", title: "Setup Database", outDegree: 1 }),
    createMockNode({ proposalId: "p2", title: "Create API", inDegree: 1, outDegree: 1 }),
    createMockNode({ proposalId: "p3", title: "Build UI", inDegree: 1 }),
  ],
  edges: [
    createMockEdge({ from: "p1", to: "p2" }),
    createMockEdge({ from: "p2", to: "p3" }),
  ],
  criticalPath: ["p1", "p2", "p3"],
  hasCycles: false,
  cycles: null,
  ...overrides,
});

const emptyGraph: DependencyGraph = {
  nodes: [],
  edges: [],
  criticalPath: [],
  hasCycles: false,
  cycles: null,
};

const graphWithCycles: DependencyGraph = {
  nodes: [
    createMockNode({ proposalId: "c1", title: "Task A", inDegree: 1, outDegree: 1 }),
    createMockNode({ proposalId: "c2", title: "Task B", inDegree: 1, outDegree: 1 }),
  ],
  edges: [
    createMockEdge({ from: "c1", to: "c2" }),
    createMockEdge({ from: "c2", to: "c1" }),
  ],
  criticalPath: [],
  hasCycles: true,
  cycles: [["c1", "c2"]],
};

describe("DependencyVisualization", () => {
  const defaultProps = {
    graph: createMockGraph(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.getByTestId("dependency-visualization")).toBeInTheDocument();
    });

    it("renders all nodes", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.getByText("Setup Database")).toBeInTheDocument();
      expect(screen.getByText("Create API")).toBeInTheDocument();
      expect(screen.getByText("Build UI")).toBeInTheDocument();
    });

    it("renders node containers for each proposal", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes).toHaveLength(3);
    });

    it("renders SVG for edges", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.getByTestId("dependency-edges")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Node Display
  // ==========================================================================

  describe("node display", () => {
    it("displays node title", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(within(nodes[0]).getByText("Setup Database")).toBeInTheDocument();
    });

    it("shows inDegree and outDegree for nodes", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      // p1 has outDegree 1
      expect(nodes[0]).toHaveAttribute("data-out-degree", "1");
      // p2 has inDegree 1, outDegree 1
      expect(nodes[1]).toHaveAttribute("data-in-degree", "1");
      expect(nodes[1]).toHaveAttribute("data-out-degree", "1");
      // p3 has inDegree 1
      expect(nodes[2]).toHaveAttribute("data-in-degree", "1");
    });

    it("marks nodes on critical path", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveAttribute("data-critical-path", "true");
      expect(nodes[1]).toHaveAttribute("data-critical-path", "true");
      expect(nodes[2]).toHaveAttribute("data-critical-path", "true");
    });

    it("marks nodes not on critical path correctly", () => {
      const graph = createMockGraph({ criticalPath: ["p1", "p3"] });
      render(<DependencyVisualization graph={graph} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[1]).toHaveAttribute("data-critical-path", "false");
    });
  });

  // ==========================================================================
  // Edge Lines
  // ==========================================================================

  describe("edge lines", () => {
    it("renders edge lines for dependencies", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const edges = screen.getAllByTestId("dependency-edge");
      expect(edges).toHaveLength(2);
    });

    it("edge has from and to data attributes", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const edges = screen.getAllByTestId("dependency-edge");
      expect(edges[0]).toHaveAttribute("data-from", "p1");
      expect(edges[0]).toHaveAttribute("data-to", "p2");
    });

    it("marks edges on critical path", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const edges = screen.getAllByTestId("dependency-edge");
      // Both edges are on critical path (p1->p2, p2->p3)
      expect(edges[0]).toHaveAttribute("data-critical-path", "true");
      expect(edges[1]).toHaveAttribute("data-critical-path", "true");
    });

    it("marks edges not on critical path", () => {
      const graph = createMockGraph({
        nodes: [
          createMockNode({ proposalId: "a", title: "A", outDegree: 1 }),
          createMockNode({ proposalId: "b", title: "B", inDegree: 1 }),
        ],
        edges: [createMockEdge({ from: "a", to: "b" })],
        criticalPath: [], // empty critical path
      });
      render(<DependencyVisualization graph={graph} />);
      const edges = screen.getAllByTestId("dependency-edge");
      expect(edges[0]).toHaveAttribute("data-critical-path", "false");
    });
  });

  // ==========================================================================
  // Critical Path Highlighting
  // ==========================================================================

  describe("critical path highlighting", () => {
    it("highlights nodes on critical path with accent color", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      // All nodes are on critical path - check via style attribute
      expect(nodes[0].getAttribute("style")).toContain("border-color: var(--accent-primary)");
    });

    it("uses muted color for non-critical nodes", () => {
      const graph = createMockGraph({ criticalPath: ["p1"] });
      render(<DependencyVisualization graph={graph} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[1].getAttribute("style")).toContain("border-color: var(--border-subtle)");
      expect(nodes[2].getAttribute("style")).toContain("border-color: var(--border-subtle)");
    });

    it("shows critical path indicator", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.getByTestId("critical-path-indicator")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Cycle Warning
  // ==========================================================================

  describe("cycle warning", () => {
    it("shows cycle warning when graph has cycles", () => {
      render(<DependencyVisualization graph={graphWithCycles} />);
      expect(screen.getByTestId("cycle-warning")).toBeInTheDocument();
    });

    it("does not show cycle warning when no cycles", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.queryByTestId("cycle-warning")).not.toBeInTheDocument();
    });

    it("displays cycle warning message", () => {
      render(<DependencyVisualization graph={graphWithCycles} />);
      expect(screen.getByText(/circular dependency/i)).toBeInTheDocument();
    });

    it("highlights nodes involved in cycles", () => {
      render(<DependencyVisualization graph={graphWithCycles} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveAttribute("data-in-cycle", "true");
      expect(nodes[1]).toHaveAttribute("data-in-cycle", "true");
    });

    it("uses error color for cycle warning", () => {
      render(<DependencyVisualization graph={graphWithCycles} />);
      const warning = screen.getByTestId("cycle-warning");
      expect(warning).toHaveStyle({ color: "var(--status-error)" });
    });
  });

  // ==========================================================================
  // Compact Mode
  // ==========================================================================

  describe("compact mode", () => {
    it("renders in compact mode when prop is true", () => {
      render(<DependencyVisualization {...defaultProps} compact />);
      const container = screen.getByTestId("dependency-visualization");
      expect(container).toHaveAttribute("data-compact", "true");
    });

    it("uses smaller node sizing in compact mode", () => {
      render(<DependencyVisualization {...defaultProps} compact />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveClass("compact-node");
    });

    it("shows truncated titles in compact mode", () => {
      render(<DependencyVisualization {...defaultProps} compact />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveClass("truncate");
    });

    it("hides degree info in compact mode", () => {
      render(<DependencyVisualization {...defaultProps} compact />);
      expect(screen.queryByTestId("degree-info")).not.toBeInTheDocument();
    });

    it("shows degree info in non-compact mode", () => {
      render(<DependencyVisualization {...defaultProps} />);
      expect(screen.getAllByTestId("degree-info").length).toBeGreaterThan(0);
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows empty message when no nodes", () => {
      render(<DependencyVisualization graph={emptyGraph} />);
      expect(screen.getByText(/no dependencies/i)).toBeInTheDocument();
    });

    it("does not render SVG when no edges", () => {
      render(<DependencyVisualization graph={emptyGraph} />);
      expect(screen.queryByTestId("dependency-edges")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has accessible name for visualization", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const container = screen.getByTestId("dependency-visualization");
      expect(container).toHaveAttribute("aria-label", "Dependency Graph");
    });

    it("SVG has accessible title", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const svg = screen.getByTestId("dependency-edges");
      expect(svg).toHaveAttribute("role", "img");
      expect(svg).toHaveAttribute("aria-label", "Dependency connections");
    });

    it("cycle warning has alert role", () => {
      render(<DependencyVisualization graph={graphWithCycles} />);
      const warning = screen.getByTestId("cycle-warning");
      expect(warning).toHaveAttribute("role", "alert");
    });

    it("nodes have descriptive labels", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveAttribute("aria-label");
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const container = screen.getByTestId("dependency-visualization");
      expect(container).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses design tokens for node background", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("uses design tokens for text", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const nodes = screen.getAllByTestId("dependency-node");
      expect(nodes[0]).toHaveStyle({ color: "var(--text-primary)" });
    });

    it("uses accent color for critical path edges", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const edges = screen.getAllByTestId("dependency-edge");
      expect(edges[0].getAttribute("stroke")).toBe("var(--accent-primary)");
    });

    it("uses muted color for non-critical edges", () => {
      const graph = createMockGraph({ criticalPath: [] });
      render(<DependencyVisualization graph={graph} />);
      const edges = screen.getAllByTestId("dependency-edge");
      expect(edges[0].getAttribute("stroke")).toBe("var(--border-subtle)");
    });
  });

  // ==========================================================================
  // Layout
  // ==========================================================================

  describe("layout", () => {
    it("arranges nodes vertically by default", () => {
      render(<DependencyVisualization {...defaultProps} />);
      const container = screen.getByTestId("nodes-container");
      expect(container).toHaveClass("flex-col");
    });

    it("can arrange nodes horizontally", () => {
      render(<DependencyVisualization {...defaultProps} direction="horizontal" />);
      const container = screen.getByTestId("nodes-container");
      expect(container).toHaveClass("flex-row");
    });
  });
});
