import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalList } from "./ProposalList";
import type { TaskProposal } from "@/types/ideation";

function createMockProposal(id: string, overrides: Partial<TaskProposal> = {}): TaskProposal {
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
    onEdit: vi.fn(),
    onRemove: vi.fn(),
    onReorder: vi.fn(),
    onSortByPriority: vi.fn(),
    onClearAll: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders list and cards", () => {
    render(<ProposalList {...defaultProps} />);

    expect(screen.getByTestId("proposal-list")).toBeInTheDocument();
    expect(screen.getByTestId("proposal-card-p1")).toBeInTheDocument();
    expect(screen.getByTestId("proposal-card-p2")).toBeInTheDocument();
    expect(screen.getByTestId("proposal-card-p3")).toBeInTheDocument();
  });

  it("renders proposals in sort order", () => {
    render(<ProposalList {...defaultProps} />);

    const cards = screen.getAllByTestId(/^proposal-card-/);
    expect(cards).toHaveLength(3);
    expect(within(cards[0]).getByText("Setup auth")).toBeInTheDocument();
    expect(within(cards[1]).getByText("Implement login")).toBeInTheDocument();
    expect(within(cards[2]).getByText("Add logout")).toBeInTheDocument();
  });

  it("shows empty state when no proposals", () => {
    render(<ProposalList {...defaultProps} proposals={[]} />);

    expect(screen.getByTestId("proposal-list-empty")).toBeInTheDocument();
    expect(screen.getByText(/no proposals yet/i)).toBeInTheDocument();
    expect(screen.queryByTestId("proposal-list-toolbar")).not.toBeInTheDocument();
  });

  it("renders toolbar actions", async () => {
    const user = userEvent.setup();
    const onSortByPriority = vi.fn();
    const onClearAll = vi.fn();

    render(
      <ProposalList
        {...defaultProps}
        onSortByPriority={onSortByPriority}
        onClearAll={onClearAll}
      />
    );

    await user.click(screen.getByTestId("sort-priority-btn"));
    await user.click(screen.getByTestId("clear-all-btn"));

    expect(onSortByPriority).toHaveBeenCalled();
    expect(onClearAll).toHaveBeenCalled();
  });

  it("renders sortable wrappers", () => {
    render(<ProposalList {...defaultProps} />);

    expect(screen.getByTestId("proposal-list-sortable")).toBeInTheDocument();

    const cards = screen.getAllByTestId(/^proposal-card-/);
    cards.forEach((card) => {
      expect(card.parentElement).toHaveAttribute("data-draggable", "true");
    });
  });

  it("passes block count dependency info to cards", () => {
    render(
      <ProposalList
        {...defaultProps}
        dependencyCounts={{
          p1: { dependsOn: 0, blocks: 2 },
          p2: { dependsOn: 1, blocks: 1 },
          p3: { dependsOn: 2, blocks: 0 },
        }}
      />
    );

    const card1 = screen.getByTestId("proposal-card-p1");
    expect(within(card1).getByTestId("blocks-count")).toHaveTextContent("\u21922");

    const card3 = screen.getByTestId("proposal-card-p3");
    expect(within(card3).queryByTestId("blocks-count")).not.toBeInTheDocument();
  });

  it("has accessible list role", () => {
    render(<ProposalList {...defaultProps} />);

    expect(screen.getByRole("list")).toBeInTheDocument();
    expect(screen.getByTestId("sort-priority-btn")).toHaveAttribute("aria-label", "Sort by priority");
    expect(screen.getByTestId("clear-all-btn")).toHaveAttribute("aria-label", "Clear all");
  });
});
