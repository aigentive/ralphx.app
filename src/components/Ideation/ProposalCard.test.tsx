/**
 * ProposalCard Component Tests
 *
 * Tests for the proposal card component with:
 * - Checkbox for selection
 * - Title and description preview
 * - Priority badge
 * - Category badge
 * - Dependency info
 * - Edit and Remove actions
 * - Selected/modified states
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalCard } from "./ProposalCard";
import type { TaskProposal } from "@/types/ideation";

// ============================================================================
// Test Utilities
// ============================================================================

function createMockProposal(overrides: Partial<TaskProposal> = {}): TaskProposal {
  return {
    id: "proposal-1",
    sessionId: "session-1",
    title: "Implement user authentication",
    description: "Add JWT-based authentication system",
    category: "feature",
    steps: ["Create auth service", "Add login endpoint"],
    acceptanceCriteria: ["Users can login", "Tokens expire correctly"],
    suggestedPriority: "high",
    priorityScore: 75,
    priorityReason: "Core functionality required",
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

describe("ProposalCard", () => {
  const defaultProps = {
    proposal: createMockProposal(),
    onSelect: vi.fn(),
    onEdit: vi.fn(),
    onRemove: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  describe("rendering", () => {
    it("renders the card container", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-card-proposal-1")).toBeInTheDocument();
    });

    it("renders the title", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-title")).toHaveTextContent(
        "Implement user authentication"
      );
    });

    it("renders the description preview", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-description")).toHaveTextContent(
        "Add JWT-based authentication system"
      );
    });

    it("renders placeholder when no description", () => {
      const proposal = createMockProposal({ description: null });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("proposal-description")).toHaveTextContent(
        "No description"
      );
    });

    it("truncates long descriptions", () => {
      const longDesc = "A".repeat(200);
      const proposal = createMockProposal({ description: longDesc });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const descEl = screen.getByTestId("proposal-description");
      expect(descEl).toHaveClass("line-clamp-2");
    });
  });

  // ============================================================================
  // Checkbox Tests
  // ============================================================================

  describe("checkbox", () => {
    it("renders checkbox", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-checkbox")).toBeInTheDocument();
    });

    it("checkbox is unchecked when not selected", () => {
      const proposal = createMockProposal({ selected: false });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("proposal-checkbox")).not.toBeChecked();
    });

    it("checkbox is checked when selected", () => {
      const proposal = createMockProposal({ selected: true });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("proposal-checkbox")).toBeChecked();
    });

    it("calls onSelect when checkbox clicked", async () => {
      const user = userEvent.setup();
      const onSelect = vi.fn();
      render(<ProposalCard {...defaultProps} onSelect={onSelect} />);

      await user.click(screen.getByTestId("proposal-checkbox"));
      expect(onSelect).toHaveBeenCalledWith("proposal-1");
    });

    it("has accessible label for checkbox", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByLabelText(/select.*implement user authentication/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Priority Badge Tests
  // ============================================================================

  describe("priority badge", () => {
    it("renders priority badge with correct value", () => {
      render(<ProposalCard {...defaultProps} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveTextContent("High");
    });

    it("renders critical priority with red styling", () => {
      const proposal = createMockProposal({ suggestedPriority: "critical" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ef4444" });
    });

    it("renders high priority with orange styling", () => {
      const proposal = createMockProposal({ suggestedPriority: "high" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ff6b35" });
    });

    it("renders medium priority with amber styling", () => {
      const proposal = createMockProposal({ suggestedPriority: "medium" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ffa94d" });
    });

    it("renders low priority with gray styling", () => {
      const proposal = createMockProposal({ suggestedPriority: "low" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#6b7280" });
    });

    it("shows user priority when set", () => {
      const proposal = createMockProposal({
        suggestedPriority: "low",
        userPriority: "critical",
      });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveTextContent("Critical");
      expect(badge).toHaveStyle({ backgroundColor: "#ef4444" });
    });
  });

  // ============================================================================
  // Category Badge Tests
  // ============================================================================

  describe("category badge", () => {
    it("renders category badge", () => {
      render(<ProposalCard {...defaultProps} />);
      const badge = screen.getByTestId("category-badge");
      expect(badge).toHaveTextContent("feature");
    });

    it("displays correct category for setup", () => {
      const proposal = createMockProposal({ category: "setup" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("category-badge")).toHaveTextContent("setup");
    });

    it("displays correct category for testing", () => {
      const proposal = createMockProposal({ category: "testing" });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("category-badge")).toHaveTextContent("testing");
    });
  });

  // ============================================================================
  // Inline Dependency Display Tests (Phase 40 - Task 4)
  // ============================================================================

  describe("inline dependency display", () => {
    const singleDep = [
      { proposalId: "dep-1", title: "Setup Database", reason: "Needs schema first" },
    ];

    const twoDeps = [
      { proposalId: "dep-1", title: "Setup Database", reason: "Needs schema first" },
      { proposalId: "dep-2", title: "Define Types" },
    ];

    const threeDeps = [
      { proposalId: "dep-1", title: "Setup Database", reason: "Needs schema first" },
      { proposalId: "dep-2", title: "Define Types" },
      { proposalId: "dep-3", title: "Create Config", reason: "Config required" },
    ];

    const fiveDeps = [
      { proposalId: "dep-1", title: "Setup Database" },
      { proposalId: "dep-2", title: "Define Types" },
      { proposalId: "dep-3", title: "Create Config" },
      { proposalId: "dep-4", title: "Add Middleware" },
      { proposalId: "dep-5", title: "Setup Auth" },
    ];

    it("does not show dependency section when no dependencies", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.queryByTestId("depends-on-inline")).not.toBeInTheDocument();
    });

    it("shows single dependency name inline with arrow", () => {
      render(<ProposalCard {...defaultProps} dependsOnDetails={singleDep} dependsOnCount={1} />);
      const inline = screen.getByTestId("depends-on-inline");
      expect(inline).toHaveTextContent("← Setup Database");
    });

    it("shows two dependency names comma-separated", () => {
      render(<ProposalCard {...defaultProps} dependsOnDetails={twoDeps} dependsOnCount={2} />);
      const inline = screen.getByTestId("depends-on-inline");
      expect(inline).toHaveTextContent("← Setup Database, Define Types");
    });

    it("truncates with +N more when more than 2 dependencies", () => {
      render(<ProposalCard {...defaultProps} dependsOnDetails={threeDeps} dependsOnCount={3} />);
      const inline = screen.getByTestId("depends-on-inline");
      expect(inline).toHaveTextContent("← Setup Database, Define Types");
      expect(inline).toHaveTextContent("+1 more");
    });

    it("shows correct count for 5 dependencies (+3 more)", () => {
      render(<ProposalCard {...defaultProps} dependsOnDetails={fiveDeps} dependsOnCount={5} />);
      const inline = screen.getByTestId("depends-on-inline");
      expect(inline).toHaveTextContent("+3 more");
    });

    it("renders expand button when dependencies exist", () => {
      render(<ProposalCard {...defaultProps} dependsOnDetails={twoDeps} dependsOnCount={2} />);
      expect(screen.getByTestId("expand-dependencies")).toBeInTheDocument();
    });

    it("expands to show full dependency list on click", async () => {
      const user = userEvent.setup();
      render(<ProposalCard {...defaultProps} dependsOnDetails={threeDeps} dependsOnCount={3} />);

      const expandBtn = screen.getByTestId("expand-dependencies");
      await user.click(expandBtn);

      const expanded = screen.getByTestId("dependencies-expanded");
      expect(expanded).toBeInTheDocument();
      expect(expanded).toHaveTextContent("Setup Database");
      expect(expanded).toHaveTextContent("Define Types");
      expect(expanded).toHaveTextContent("Create Config");
    });

    it("shows reason text in expanded view when available", async () => {
      const user = userEvent.setup();
      render(<ProposalCard {...defaultProps} dependsOnDetails={threeDeps} dependsOnCount={3} />);

      const expandBtn = screen.getByTestId("expand-dependencies");
      await user.click(expandBtn);

      const expanded = screen.getByTestId("dependencies-expanded");
      expect(expanded).toHaveTextContent("Needs schema first");
      expect(expanded).toHaveTextContent("Config required");
    });

    it("collapses expanded section on second click", async () => {
      const user = userEvent.setup();
      render(<ProposalCard {...defaultProps} dependsOnDetails={twoDeps} dependsOnCount={2} />);

      const expandBtn = screen.getByTestId("expand-dependencies");
      await user.click(expandBtn);
      expect(screen.getByTestId("dependencies-expanded")).toBeInTheDocument();

      await user.click(expandBtn);
      expect(screen.queryByTestId("dependencies-expanded")).not.toBeInTheDocument();
    });

    it("shows blocks count badge (unchanged from before)", () => {
      render(<ProposalCard {...defaultProps} blocksCount={3} />);
      expect(screen.getByTestId("blocks-count")).toHaveTextContent("→3");
    });

    it("does not show blocks badge when blocksCount is 0", () => {
      render(<ProposalCard {...defaultProps} blocksCount={0} />);
      expect(screen.queryByTestId("blocks-count")).not.toBeInTheDocument();
    });

    it("does not show blocks badge when blocksCount is undefined", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.queryByTestId("blocks-count")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Action Button Tests
  // ============================================================================

  describe("action buttons", () => {
    it("renders edit button", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-edit")).toBeInTheDocument();
    });

    it("renders remove button", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByTestId("proposal-remove")).toBeInTheDocument();
    });

    it("calls onEdit when edit clicked", async () => {
      const user = userEvent.setup();
      const onEdit = vi.fn();
      render(<ProposalCard {...defaultProps} onEdit={onEdit} />);

      await user.click(screen.getByTestId("proposal-edit"));
      expect(onEdit).toHaveBeenCalledWith("proposal-1");
    });

    it("calls onRemove when remove clicked", async () => {
      const user = userEvent.setup();
      const onRemove = vi.fn();
      render(<ProposalCard {...defaultProps} onRemove={onRemove} />);

      await user.click(screen.getByTestId("proposal-remove"));
      expect(onRemove).toHaveBeenCalledWith("proposal-1");
    });

    it("edit button has accessible label", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByLabelText(/edit proposal/i)).toBeInTheDocument();
    });

    it("remove button has accessible label", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByLabelText(/remove proposal/i)).toBeInTheDocument();
    });

    it("action buttons are visible on hover (group-hover)", () => {
      render(<ProposalCard {...defaultProps} />);
      const actions = screen.getByTestId("proposal-actions");
      expect(actions).toHaveClass("opacity-0", "group-hover:opacity-100");
    });
  });

  // ============================================================================
  // Selected State Tests
  // ============================================================================

  describe("selected state", () => {
    it("applies orange border when selected", () => {
      const proposal = createMockProposal({ selected: true });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      expect(card).toHaveStyle({ borderColor: "#ff6b35" });
    });

    it("applies subtle border when not selected", () => {
      const proposal = createMockProposal({ selected: false });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      // When not selected, borderWidth is 1px (not 2px like selected)
      expect(card).toHaveStyle({ borderWidth: "1px" });
    });
  });

  // ============================================================================
  // Modified Indicator Tests
  // ============================================================================

  describe("modified indicator", () => {
    it("does not show modified indicator when not modified", () => {
      const proposal = createMockProposal({ userModified: false });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.queryByTestId("modified-indicator")).not.toBeInTheDocument();
    });

    it("shows modified indicator when user modified", () => {
      const proposal = createMockProposal({ userModified: true });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("modified-indicator")).toBeInTheDocument();
    });

    it("modified indicator has correct text", () => {
      const proposal = createMockProposal({ userModified: true });
      render(<ProposalCard {...defaultProps} proposal={proposal} />);
      expect(screen.getByTestId("modified-indicator")).toHaveTextContent("Modified");
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("accessibility", () => {
    it("has article role", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.getByRole("article")).toBeInTheDocument();
    });

    it("has aria-labelledby pointing to title", () => {
      render(<ProposalCard {...defaultProps} />);
      const article = screen.getByRole("article");
      expect(article).toHaveAttribute("aria-labelledby", "proposal-title-proposal-1");
    });

    it("checkbox is keyboard accessible", async () => {
      const user = userEvent.setup();
      const onSelect = vi.fn();
      render(<ProposalCard {...defaultProps} onSelect={onSelect} />);

      const checkbox = screen.getByTestId("proposal-checkbox");
      await user.click(checkbox);

      expect(onSelect).toHaveBeenCalled();
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("styling", () => {
    it("applies dark surface background", () => {
      render(<ProposalCard {...defaultProps} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      expect(card).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("has rounded corners", () => {
      render(<ProposalCard {...defaultProps} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      expect(card).toHaveClass("rounded-lg");
    });

    it("has border", () => {
      render(<ProposalCard {...defaultProps} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      expect(card).toHaveClass("border");
    });

    it("has transition for interactions", () => {
      render(<ProposalCard {...defaultProps} />);
      const card = screen.getByTestId("proposal-card-proposal-1");
      expect(card).toHaveClass("transition-all");
    });
  });

  // ============================================================================
  // Complexity Tests
  // ============================================================================

  describe("complexity indicator", () => {
    it("shows complexity when provided", () => {
      render(<ProposalCard {...defaultProps} showComplexity />);
      expect(screen.getByTestId("complexity-indicator")).toBeInTheDocument();
    });

    it("displays correct complexity value", () => {
      const proposal = createMockProposal({ estimatedComplexity: "complex" });
      render(<ProposalCard {...defaultProps} proposal={proposal} showComplexity />);
      expect(screen.getByTestId("complexity-indicator")).toHaveTextContent("complex");
    });

    it("does not show complexity by default", () => {
      render(<ProposalCard {...defaultProps} />);
      expect(screen.queryByTestId("complexity-indicator")).not.toBeInTheDocument();
    });
  });
});
