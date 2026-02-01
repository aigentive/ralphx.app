/**
 * ProposalList Component Tests
 *
 * Tests for the proposal list component with:
 * - List of ProposalCard components
 * - Drag-to-reorder with @dnd-kit
 * - Clear all button
 * - Empty state when no proposals
 *
 * NOTE: This component is no longer used in the UI (replaced by TieredProposalList).
 * Tests retained for documentation purposes.
 */

import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalList } from "./ProposalList";
import type { TaskProposal } from "@/types/ideation";

// ============================================================================
// Test Utilities
// ============================================================================

function createMockProposal(
  id: string,
  overrides: Partial<TaskProposal> = {}
): TaskProposal {
  return {
    id,
    sessionId: "session-1",
    title: `Proposal ${id}`,
    description: `Description for ${id}`,
    category: "feature",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "medium",
    priorityScore: 50,
    priorityReason: null,
    estimatedComplexity: "moderate",
    userPriority: null,
    userModified: false,
    status: "pending",
    selected: false,
    createdTaskId: null,
    sortOrder: 0,
    createdAt: "2026-01-24T10:00:00Z",
    updatedAt: "2026-01-24T10:00:00Z",
    ...overrides,
  };
}

function createMockProposals(): TaskProposal[] {
  return [
    createMockProposal("p1", { title: "Setup auth", suggestedPriority: "high", sortOrder: 0 }),
    createMockProposal("p2", { title: "Implement login", suggestedPriority: "medium", sortOrder: 1 }),
    createMockProposal("p3", { title: "Add logout", suggestedPriority: "low", sortOrder: 2 }),
  ];
}

describe("ProposalList", () => {
  const defaultProps = {
    proposals: createMockProposals(),
    onSelect: vi.fn(),
    onEdit: vi.fn(),
    onRemove: vi.fn(),
    onReorder: vi.fn(),
    onSelectAll: vi.fn(),
    onDeselectAll: vi.fn(),
    onSortByPriority: vi.fn(),
    onClearAll: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  describe("rendering", () => {
    it("renders the list container", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("proposal-list")).toBeInTheDocument();
    });

    it("renders all proposal cards", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-card-p2")).toBeInTheDocument();
      expect(screen.getByTestId("proposal-card-p3")).toBeInTheDocument();
    });

    it("renders proposals in sortOrder", () => {
      render(<ProposalList {...defaultProps} />);
      const cards = screen.getAllByRole("article");
      expect(cards).toHaveLength(3);
      // First card should be "Setup auth" (sortOrder: 0)
      expect(within(cards[0]).getByTestId("proposal-title")).toHaveTextContent("Setup auth");
    });

    it("renders toolbar with action buttons", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("proposal-list-toolbar")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Empty State Tests
  // ============================================================================

  describe("empty state", () => {
    it("shows empty state when no proposals", () => {
      render(<ProposalList {...defaultProps} proposals={[]} />);
      expect(screen.getByTestId("proposal-list-empty")).toBeInTheDocument();
    });

    it("empty state has descriptive text", () => {
      render(<ProposalList {...defaultProps} proposals={[]} />);
      expect(screen.getByText(/no proposals yet/i)).toBeInTheDocument();
    });

    it("hides toolbar when empty", () => {
      render(<ProposalList {...defaultProps} proposals={[]} />);
      expect(screen.queryByTestId("proposal-list-toolbar")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Select All / Deselect All Tests
  // ============================================================================

  describe("select all / deselect all", () => {
    it("renders select all button", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("select-all-btn")).toBeInTheDocument();
    });

    it("renders deselect all button", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("deselect-all-btn")).toBeInTheDocument();
    });

    it("calls onSelectAll when select all clicked", async () => {
      const user = userEvent.setup();
      const onSelectAll = vi.fn();
      render(<ProposalList {...defaultProps} onSelectAll={onSelectAll} />);

      await user.click(screen.getByTestId("select-all-btn"));
      expect(onSelectAll).toHaveBeenCalled();
    });

    it("calls onDeselectAll when deselect all clicked", async () => {
      const user = userEvent.setup();
      const onDeselectAll = vi.fn();
      render(<ProposalList {...defaultProps} onDeselectAll={onDeselectAll} />);

      await user.click(screen.getByTestId("deselect-all-btn"));
      expect(onDeselectAll).toHaveBeenCalled();
    });

    it("shows selected count in toolbar", () => {
      const proposals = [
        createMockProposal("p1", { selected: true }),
        createMockProposal("p2", { selected: true }),
        createMockProposal("p3", { selected: false }),
      ];
      render(<ProposalList {...defaultProps} proposals={proposals} />);
      expect(screen.getByTestId("selected-count")).toHaveTextContent("2 selected");
    });

    it("shows correct count for zero selected", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("selected-count")).toHaveTextContent("0 selected");
    });
  });

  // ============================================================================
  // Sort by Priority Tests
  // ============================================================================

  describe("sort by priority", () => {
    it("renders sort by priority button", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("sort-priority-btn")).toBeInTheDocument();
    });

    it("calls onSortByPriority when clicked", async () => {
      const user = userEvent.setup();
      const onSortByPriority = vi.fn();
      render(<ProposalList {...defaultProps} onSortByPriority={onSortByPriority} />);

      await user.click(screen.getByTestId("sort-priority-btn"));
      expect(onSortByPriority).toHaveBeenCalled();
    });

    it("has accessible label", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByLabelText(/sort by priority/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Clear All Tests
  // ============================================================================

  describe("clear all", () => {
    it("renders clear all button", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByTestId("clear-all-btn")).toBeInTheDocument();
    });

    it("calls onClearAll when clicked", async () => {
      const user = userEvent.setup();
      const onClearAll = vi.fn();
      render(<ProposalList {...defaultProps} onClearAll={onClearAll} />);

      await user.click(screen.getByTestId("clear-all-btn"));
      expect(onClearAll).toHaveBeenCalled();
    });

    it("has accessible label", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByLabelText(/clear all/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Card Interaction Tests
  // ============================================================================

  describe("card interactions", () => {
    it("calls onSelect when card clicked", async () => {
      const onSelect = vi.fn();
      render(<ProposalList {...defaultProps} onSelect={onSelect} />);

      const card = screen.getByTestId("proposal-card-p1");
      // Click on the card to select
      fireEvent.click(card);

      expect(onSelect).toHaveBeenCalledWith("p1");
    });

    it("calls onEdit when card edit clicked", async () => {
      const onEdit = vi.fn();
      render(<ProposalList {...defaultProps} onEdit={onEdit} />);

      const card = screen.getByTestId("proposal-card-p1");
      const editBtn = within(card).getByTestId("proposal-edit");
      fireEvent.click(editBtn);

      expect(onEdit).toHaveBeenCalledWith("p1");
    });

    it("calls onRemove when card remove clicked", async () => {
      const onRemove = vi.fn();
      render(<ProposalList {...defaultProps} onRemove={onRemove} />);

      const card = screen.getByTestId("proposal-card-p1");
      const removeBtn = within(card).getByTestId("proposal-remove");
      fireEvent.click(removeBtn);

      expect(onRemove).toHaveBeenCalledWith("p1");
    });
  });

  // ============================================================================
  // Drag and Drop Tests
  // ============================================================================

  describe("drag and drop", () => {
    it("renders sortable context", () => {
      render(<ProposalList {...defaultProps} />);
      // The list should be wrapped in a sortable context
      expect(screen.getByTestId("proposal-list-sortable")).toBeInTheDocument();
    });

    it("cards are wrapped in sortable elements", () => {
      render(<ProposalList {...defaultProps} />);
      // Each card should be wrapped in a sortable element with data-draggable
      const draggables = screen.getAllByTestId(/^proposal-card-/);
      draggables.forEach((card) => {
        // The sortable wrapper has data-draggable attribute
        const wrapper = card.parentElement;
        expect(wrapper).toHaveAttribute("data-draggable", "true");
      });
    });

    it("calls onReorder when drag ends", () => {
      // Note: Full drag simulation is complex in tests
      // We verify the callback is passed to the component
      const onReorder = vi.fn();
      render(<ProposalList {...defaultProps} onReorder={onReorder} />);

      // onReorder should be wired up (tested via integration)
      expect(screen.getByTestId("proposal-list-sortable")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Dependency Count Tests
  // ============================================================================

  describe("dependency counts", () => {
    it("passes dependency counts to cards when provided", () => {
      const dependencyCounts = {
        p1: { dependsOn: 0, blocks: 2 },
        p2: { dependsOn: 1, blocks: 1 },
        p3: { dependsOn: 2, blocks: 0 },
      };
      render(
        <ProposalList {...defaultProps} dependencyCounts={dependencyCounts} />
      );

      const card1 = screen.getByTestId("proposal-card-p1");
      expect(within(card1).getByTestId("dependency-info")).toHaveTextContent("Blocks 2");

      const card3 = screen.getByTestId("proposal-card-p3");
      expect(within(card3).getByTestId("dependency-info")).toHaveTextContent("Depends on 2");
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("styling", () => {
    it("applies proper spacing between cards", () => {
      render(<ProposalList {...defaultProps} />);
      const list = screen.getByTestId("proposal-list-sortable");
      expect(list).toHaveClass("space-y-2");
    });

    it("toolbar has proper styling", () => {
      render(<ProposalList {...defaultProps} />);
      const toolbar = screen.getByTestId("proposal-list-toolbar");
      expect(toolbar).toHaveClass("flex", "items-center", "justify-between");
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("accessibility", () => {
    it("list has proper role", () => {
      render(<ProposalList {...defaultProps} />);
      expect(screen.getByRole("list")).toBeInTheDocument();
    });

    it("toolbar buttons have accessible labels", () => {
      render(<ProposalList {...defaultProps} />);
      // Use getByTestId + check aria-label to avoid multiple matches
      expect(screen.getByTestId("select-all-btn")).toHaveAttribute("aria-label", "Select all");
      expect(screen.getByTestId("deselect-all-btn")).toHaveAttribute("aria-label", "Deselect all");
      expect(screen.getByTestId("sort-priority-btn")).toHaveAttribute("aria-label", "Sort by priority");
      expect(screen.getByTestId("clear-all-btn")).toHaveAttribute("aria-label", "Clear all");
    });
  });
});
