/**
 * TieredProposalList Component Tests
 *
 * Tests for the tiered proposal list orchestration component with:
 * - Tier grouping based on dependency graph
 * - Proposal rendering within tiers
 * - Dependency details passed to ProposalCard
 * - Critical path highlighting
 * - Sort order preservation within tiers
 */

import { render, screen, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { TieredProposalList } from "./TieredProposalList";
import type { TaskProposal, DependencyGraph } from "@/types/ideation";

// ============================================================================
// Test Fixtures
// ============================================================================

const createProposal = (overrides: Partial<TaskProposal> = {}): TaskProposal => ({
  id: "proposal-1",
  sessionId: "session-1",
  title: "Test Proposal",
  description: "Test description",
  category: "frontend",
  steps: ["Step 1"],
  acceptanceCriteria: ["Criterion 1"],
  suggestedPriority: "medium",
  priorityScore: 50,
  priorityReason: null,
  estimatedComplexity: "moderate",
  userPriority: null,
  userModified: false,
  status: "pending",
  selected: false,
  createdTaskId: null,
  planArtifactId: null,
  planVersionAtCreation: null,
  sortOrder: 0,
  createdAt: "2026-01-31T10:00:00Z",
  updatedAt: "2026-01-31T10:00:00Z",
  ...overrides,
});

const createDependencyGraph = (
  nodes: Array<{ proposalId: string; title: string; inDegree: number; outDegree: number }>,
  edges: Array<{ from: string; to: string; reason?: string }> = [],
  criticalPath: string[] = []
): DependencyGraph => ({
  nodes,
  edges,
  criticalPath,
  hasCycles: false,
  cycles: null,
});

const defaultProps = {
  proposals: [],
  dependencyGraph: null,
  highlightedIds: new Set<string>(),
  criticalPathIds: new Set<string>(),
  onSelect: vi.fn(),
  onEdit: vi.fn(),
  onRemove: vi.fn(),
};

// ============================================================================
// Component Tests
// ============================================================================

describe("TieredProposalList", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Empty State Tests
  // ============================================================================

  describe("empty state", () => {
    it("returns null when no proposals", () => {
      const { container } = render(<TieredProposalList {...defaultProps} />);
      expect(container.firstChild).toBeNull();
    });

    it("returns null when proposals array is empty", () => {
      const { container } = render(
        <TieredProposalList {...defaultProps} proposals={[]} />
      );
      expect(container.firstChild).toBeNull();
    });
  });

  // ============================================================================
  // Basic Rendering Tests
  // ============================================================================

  describe("basic rendering", () => {
    it("renders tiered-proposal-list container", () => {
      const proposals = [createProposal()];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByTestId("tiered-proposal-list")).toBeInTheDocument();
    });

    it("renders proposal card for single proposal", () => {
      const proposals = [createProposal({ id: "p1", title: "First Proposal" })];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
      expect(screen.getByText("First Proposal")).toBeInTheDocument();
    });

    it("renders multiple proposals", () => {
      const proposals = [
        createProposal({ id: "p1", title: "First" }),
        createProposal({ id: "p2", title: "Second" }),
      ];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-card-p2")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Tier Grouping Tests
  // ============================================================================

  describe("tier grouping", () => {
    it("renders tier 0 group when no dependencies", () => {
      const proposals = [createProposal({ id: "p1" })];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByTestId("proposal-tier-group-0")).toBeInTheDocument();
      expect(screen.getByText("Foundation")).toBeInTheDocument();
    });

    it("groups proposals into correct tiers based on dependencies", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation Task", sortOrder: 0 }),
        createProposal({ id: "p2", title: "Core Task", sortOrder: 1 }),
        createProposal({ id: "p3", title: "Integration Task", sortOrder: 2 }),
      ];

      // Edge semantics in computeDependencyTiers: edge.to depends on edge.from
      // So { from: "p1", to: "p2" } means p2 depends on p1
      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation Task", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Core Task", inDegree: 1, outDegree: 1 },
          { proposalId: "p3", title: "Integration Task", inDegree: 1, outDegree: 0 },
        ],
        [
          { from: "p1", to: "p2" }, // p2 depends on p1
          { from: "p2", to: "p3" }, // p3 depends on p2
        ]
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Should have three tiers
      expect(screen.getByTestId("proposal-tier-group-0")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-tier-group-1")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-tier-group-2")).toBeInTheDocument();

      // Verify tier labels
      expect(screen.getByText("Foundation")).toBeInTheDocument();
      expect(screen.getByText("Core")).toBeInTheDocument();
      expect(screen.getByText("Integration")).toBeInTheDocument();
    });

    it("shows correct proposal count in tier groups", () => {
      const proposals = [
        createProposal({ id: "p1", sortOrder: 0 }),
        createProposal({ id: "p2", sortOrder: 1 }),
        createProposal({ id: "p3", sortOrder: 2 }),
      ];

      // All in tier 0 (no dependencies)
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByText("3 proposals")).toBeInTheDocument();
    });

    it("shows singular 'proposal' for single proposal in tier", () => {
      const proposals = [createProposal({ id: "p1" })];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByText("1 proposal")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Sort Order Tests
  // ============================================================================

  describe("sort order within tiers", () => {
    it("maintains sortOrder within same tier", () => {
      const proposals = [
        createProposal({ id: "p3", title: "Third", sortOrder: 2 }),
        createProposal({ id: "p1", title: "First", sortOrder: 0 }),
        createProposal({ id: "p2", title: "Second", sortOrder: 1 }),
      ];

      render(<TieredProposalList {...defaultProps} proposals={proposals} />);

      // Get all proposal cards and verify order
      const cards = screen.getAllByTestId(/^proposal-card-/);
      expect(cards).toHaveLength(3);

      // Cards should be in sortOrder order (0, 1, 2)
      expect(within(cards[0]).getByText("First")).toBeInTheDocument();
      expect(within(cards[1]).getByText("Second")).toBeInTheDocument();
      expect(within(cards[2]).getByText("Third")).toBeInTheDocument();
    });

    it("maintains sortOrder within each tier separately", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation 2", sortOrder: 1 }),
        createProposal({ id: "p2", title: "Foundation 1", sortOrder: 0 }),
        createProposal({ id: "p3", title: "Core 1", sortOrder: 0 }),
      ];

      // Edge semantics: edge.to depends on edge.from
      // { from: "p2", to: "p3" } means p3 depends on p2
      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation 2", inDegree: 0, outDegree: 0 },
          { proposalId: "p2", title: "Foundation 1", inDegree: 0, outDegree: 1 },
          { proposalId: "p3", title: "Core 1", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p2", to: "p3" }] // p3 depends on p2
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Get tier 0 group and verify order within it
      const tier0 = screen.getByTestId("proposal-tier-group-0");
      const tier0Cards = within(tier0).getAllByTestId(/^proposal-card-/);
      expect(tier0Cards).toHaveLength(2);
      expect(within(tier0Cards[0]).getByText("Foundation 1")).toBeInTheDocument();
      expect(within(tier0Cards[1]).getByText("Foundation 2")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Dependency Details Tests
  // ============================================================================

  describe("dependency details", () => {
    it("passes dependency details to ProposalCard", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Base Task" }),
        createProposal({ id: "p2", title: "Dependent Task" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Base Task", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Dependent Task", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p2", to: "p1", reason: "Needs base implementation" }]
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // The dependent card should show dependency info
      const dependentCard = screen.getByTestId("proposal-card-p2");
      // ProposalCard shows "← Title" for dependencies
      expect(within(dependentCard).getByText(/Base Task/)).toBeInTheDocument();
    });

    it("shows blocks count for proposals that block others", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Base Task" }),
        createProposal({ id: "p2", title: "Dependent Task" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Base Task", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Dependent Task", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p2", to: "p1" }] // p2 depends on p1, so p1 blocks 1
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Base task should show it blocks 1 proposal
      const baseCard = screen.getByTestId("proposal-card-p1");
      expect(within(baseCard).getByTestId("blocks-count")).toHaveTextContent("→1");
    });
  });

  // ============================================================================
  // Critical Path Tests
  // ============================================================================

  describe("critical path", () => {
    it("passes critical path status to ProposalCard", () => {
      const proposals = [createProposal({ id: "p1", title: "Critical Task" })];
      const criticalPathIds = new Set(["p1"]);

      render(
        <TieredProposalList
          {...defaultProps}
          proposals={proposals}
          criticalPathIds={criticalPathIds}
        />
      );

      // Card should have critical path styling (bottom border)
      const card = screen.getByTestId("proposal-card-p1");
      expect(card.className).toContain("border-b-");
    });
  });

  // ============================================================================
  // Highlighting Tests
  // ============================================================================

  describe("highlighting", () => {
    it("passes highlighted status to ProposalCard", () => {
      const proposals = [createProposal({ id: "p1" })];
      const highlightedIds = new Set(["p1"]);

      render(
        <TieredProposalList
          {...defaultProps}
          proposals={proposals}
          highlightedIds={highlightedIds}
        />
      );

      // Card should have highlight styling
      const card = screen.getByTestId("proposal-card-p1");
      expect(card.className).toContain("border-yellow");
    });
  });

  // ============================================================================
  // Callback Tests
  // ============================================================================

  describe("callbacks", () => {
    it("calls onSelect when proposal is selected", async () => {
      const onSelect = vi.fn();
      const proposals = [createProposal({ id: "p1" })];

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} onSelect={onSelect} />
      );

      const card = screen.getByTestId("proposal-card-p1");
      card.click();

      expect(onSelect).toHaveBeenCalledWith("p1");
    });

    it("calls onEdit when edit button is clicked", async () => {
      const onEdit = vi.fn();
      const proposals = [createProposal({ id: "p1" })];

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} onEdit={onEdit} />
      );

      // The buttons are hidden until hover but still in the DOM
      // Find all buttons in the card and click the one that triggers onEdit
      const card = screen.getByTestId("proposal-card-p1");
      const buttons = within(card).getAllByRole("button");

      // The edit button is typically the second button (after checkbox toggle if it's a button)
      // or we can find it by looking for the icon - lucide uses format lucide-{name}
      // FileEdit in lucide-react renders as lucide-file-edit class
      // Let's just click buttons until we find the right one
      for (const button of buttons) {
        button.click();
        if (onEdit.mock.calls.length > 0) break;
      }

      expect(onEdit).toHaveBeenCalledWith("p1");
    });

    it("calls onRemove when remove button is clicked", async () => {
      const onRemove = vi.fn();
      const proposals = [createProposal({ id: "p1" })];

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} onRemove={onRemove} />
      );

      // Find button by its icon (Trash2) - it's the second action button
      const card = screen.getByTestId("proposal-card-p1");
      const buttons = within(card).getAllByRole("button");
      // Find button with trash icon
      const removeButton = buttons.find(btn => btn.querySelector(".lucide-trash-2"));
      expect(removeButton).toBeDefined();
      removeButton!.click();

      expect(onRemove).toHaveBeenCalledWith("p1");
    });
  });

  // ============================================================================
  // No Dependency Graph Tests
  // ============================================================================

  describe("without dependency graph", () => {
    it("puts all proposals in tier 0 when no dependency graph", () => {
      const proposals = [
        createProposal({ id: "p1" }),
        createProposal({ id: "p2" }),
        createProposal({ id: "p3" }),
      ];

      render(<TieredProposalList {...defaultProps} proposals={proposals} />);

      // Should only have tier 0
      expect(screen.getByTestId("proposal-tier-group-0")).toBeInTheDocument();
      expect(screen.queryByTestId("proposal-tier-group-1")).not.toBeInTheDocument();

      // All three proposals should be in tier 0
      const tier0 = screen.getByTestId("proposal-tier-group-0");
      expect(within(tier0).getAllByTestId(/^proposal-card-/)).toHaveLength(3);
    });
  });

  // ============================================================================
  // Tier Connector Tests
  // ============================================================================

  describe("tier connectors", () => {
    it("does not render connector before first tier", () => {
      const proposals = [createProposal({ id: "p1" })];
      render(<TieredProposalList {...defaultProps} proposals={proposals} />);

      // Should not have any connectors with single tier
      expect(screen.queryByTestId("tier-connector")).not.toBeInTheDocument();
    });

    it("renders connector between tiers", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation" }),
        createProposal({ id: "p2", title: "Core" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Core", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p1", to: "p2" }] // p2 depends on p1 → two tiers
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Should have one connector between tier 0 and tier 1
      expect(screen.getByTestId("tier-connector")).toBeInTheDocument();
    });

    it("renders multiple connectors for multiple tier transitions", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation Task" }),
        createProposal({ id: "p2", title: "Core Task" }),
        createProposal({ id: "p3", title: "Integration Task" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation Task", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Core Task", inDegree: 1, outDegree: 1 },
          { proposalId: "p3", title: "Integration Task", inDegree: 1, outDegree: 0 },
        ],
        [
          { from: "p1", to: "p2" }, // p2 depends on p1
          { from: "p2", to: "p3" }, // p3 depends on p2
        ]
      );

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Should have two connectors (0→1, 1→2)
      const connectors = screen.getAllByTestId("tier-connector");
      expect(connectors).toHaveLength(2);
    });

    it("highlights connector on critical path", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation" }),
        createProposal({ id: "p2", title: "Core" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Core", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p1", to: "p2" }],
        ["p1", "p2"] // Both on critical path
      );

      const criticalPathIds = new Set(["p1", "p2"]);

      render(
        <TieredProposalList
          {...defaultProps}
          proposals={proposals}
          dependencyGraph={graph}
          criticalPathIds={criticalPathIds}
        />
      );

      // The connector SVG should use critical path color (#ff6b35)
      const connector = screen.getByTestId("tier-connector");
      const svgLine = connector.querySelector("line");
      expect(svgLine).toHaveAttribute("stroke", "#ff6b35");
    });

    it("uses dashed style for non-critical connector", () => {
      const proposals = [
        createProposal({ id: "p1", title: "Foundation" }),
        createProposal({ id: "p2", title: "Core" }),
      ];

      const graph = createDependencyGraph(
        [
          { proposalId: "p1", title: "Foundation", inDegree: 0, outDegree: 1 },
          { proposalId: "p2", title: "Core", inDegree: 1, outDegree: 0 },
        ],
        [{ from: "p1", to: "p2" }]
      );

      // No critical path
      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      const connector = screen.getByTestId("tier-connector");
      const svgLine = connector.querySelector("line");
      expect(svgLine).toHaveAttribute("stroke-dasharray", "3 2");
    });
  });

  // ============================================================================
  // Edge Case Tests
  // ============================================================================

  describe("edge cases", () => {
    it("handles proposal not in graph nodes gracefully", () => {
      const proposals = [
        createProposal({ id: "p1", title: "In Graph" }),
        createProposal({ id: "p2", title: "Not In Graph" }),
      ];

      const graph = createDependencyGraph([
        { proposalId: "p1", title: "In Graph", inDegree: 0, outDegree: 0 },
      ]);

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      // Both should render (p2 defaults to tier 0)
      expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-card-p2")).toBeInTheDocument();
    });

    it("handles empty dependency graph (no nodes/edges)", () => {
      const proposals = [createProposal({ id: "p1" })];
      const graph = createDependencyGraph([]);

      render(
        <TieredProposalList {...defaultProps} proposals={proposals} dependencyGraph={graph} />
      );

      expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
    });
  });
});
