import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalCard } from "./ProposalCard";
import type { TaskProposal } from "@/types/ideation";

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
    onEdit: vi.fn(),
    onDelete: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders basic proposal content", () => {
    render(<ProposalCard {...defaultProps} />);

    const card = screen.getByTestId("proposal-card-proposal-1");
    expect(card).toBeInTheDocument();
    expect(within(card).getByText("Implement user authentication")).toBeInTheDocument();
    expect(within(card).getByText("Add JWT-based authentication system")).toBeInTheDocument();
    expect(within(card).getByText("feature")).toBeInTheDocument();
    expect(within(card).getByText("High")).toBeInTheDocument();
  });

  it("renders fallback description when absent", () => {
    render(
      <ProposalCard
        {...defaultProps}
        proposal={createMockProposal({ description: null })}
      />
    );

    expect(screen.getByText("No description")).toBeInTheDocument();
  });

  it("uses user priority when present", () => {
    render(
      <ProposalCard
        {...defaultProps}
        proposal={createMockProposal({ userPriority: "critical", suggestedPriority: "low" })}
      />
    );

    expect(screen.getByText("Critical")).toBeInTheDocument();
    expect(screen.queryByText("Low")).not.toBeInTheDocument();
  });

  it("shows modified indicator", () => {
    render(
      <ProposalCard
        {...defaultProps}
        proposal={createMockProposal({ userModified: true })}
      />
    );

    expect(screen.getByText("Modified")).toBeInTheDocument();
  });

  it("shows and hides blocks badge", () => {
    const { rerender } = render(
      <ProposalCard {...defaultProps} blocksCount={3} />
    );

    expect(screen.getByTestId("blocks-count")).toHaveTextContent("\u21923");

    rerender(<ProposalCard {...defaultProps} blocksCount={0} />);
    expect(screen.queryByTestId("blocks-count")).not.toBeInTheDocument();
  });

  it("expands dependencies", async () => {
    const user = userEvent.setup();
    render(
      <ProposalCard
        {...defaultProps}
        dependsOnDetails={[
          { proposalId: "dep-1", title: "Setup DB", reason: "Schema first" },
          { proposalId: "dep-2", title: "Define Types" },
          { proposalId: "dep-3", title: "Config" },
        ]}
      />
    );

    expect(screen.getByTestId("depends-on-inline")).toHaveTextContent("Setup DB");
    await user.click(screen.getByTestId("depends-on-inline"));

    const expanded = screen.getByTestId("dependencies-expanded");
    expect(expanded).toHaveTextContent("Setup DB");
    expect(expanded).toHaveTextContent("Define Types");
    expect(expanded).toHaveTextContent("Config");
    expect(expanded).toHaveTextContent("Schema first");
  });

  it("calls edit and remove actions", async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();
    const onDelete = vi.fn();

    render(
      <ProposalCard
        {...defaultProps}
        onEdit={onEdit}
        onDelete={onDelete}
      />
    );

    const card = screen.getByTestId("proposal-card-proposal-1");
    const buttons = within(card).getAllByRole("button");

    await user.click(buttons[0]);
    await user.click(buttons[1]);

    expect(onEdit).toHaveBeenCalledWith("proposal-1");
    expect(onDelete).toHaveBeenCalledWith("proposal-1");
  });

  it("hides edit/remove actions in read-only mode", () => {
    render(<ProposalCard {...defaultProps} isReadOnly={true} />);
    const card = screen.getByTestId("proposal-card-proposal-1");
    expect(within(card).queryAllByRole("button")).toHaveLength(0);
  });

  it("keeps modern card styling classes", () => {
    render(<ProposalCard {...defaultProps} />);
    const card = screen.getByTestId("proposal-card-proposal-1");
    expect(card).toHaveClass("rounded-xl");
    expect(card).toHaveClass("transition-all");
  });

  it("calls onViewDetail with proposal id and enrichment when card is clicked", async () => {
    const user = userEvent.setup();
    const onViewDetail = vi.fn();
    render(
      <ProposalCard
        {...defaultProps}
        onViewDetail={onViewDetail}
        dependsOnDetails={[{ proposalId: "dep-1", title: "Dep" }]}
        blocksCount={2}
        isOnCriticalPath={true}
      />
    );
    const card = screen.getByTestId("proposal-card-proposal-1");
    await user.click(card);
    expect(onViewDetail).toHaveBeenCalledWith("proposal-1", {
      dependsOnDetails: [{ proposalId: "dep-1", title: "Dep" }],
      blocksCount: 2,
      isOnCriticalPath: true,
    });
  });

  it("does not call onViewDetail when edit button is clicked (stopPropagation)", async () => {
    const user = userEvent.setup();
    const onViewDetail = vi.fn();
    const onEdit = vi.fn();
    render(
      <ProposalCard
        {...defaultProps}
        onEdit={onEdit}
        onViewDetail={onViewDetail}
      />
    );
    const card = screen.getByTestId("proposal-card-proposal-1");
    const buttons = within(card).getAllByRole("button");
    // Click edit button — should NOT trigger card click
    await user.click(buttons[0]);
    expect(onEdit).toHaveBeenCalledWith("proposal-1");
    expect(onViewDetail).not.toHaveBeenCalled();
  });

  it("does not call onViewDetail when delete button is clicked (stopPropagation)", async () => {
    const user = userEvent.setup();
    const onViewDetail = vi.fn();
    const onDelete = vi.fn();
    render(
      <ProposalCard
        {...defaultProps}
        onDelete={onDelete}
        onViewDetail={onViewDetail}
      />
    );
    const card = screen.getByTestId("proposal-card-proposal-1");
    const buttons = within(card).getAllByRole("button");
    await user.click(buttons[1]);
    expect(onDelete).toHaveBeenCalledWith("proposal-1");
    expect(onViewDetail).not.toHaveBeenCalled();
  });

  it("adds cursor-pointer class when onViewDetail provided", () => {
    render(
      <ProposalCard
        {...defaultProps}
        onViewDetail={vi.fn()}
      />
    );
    const card = screen.getByTestId("proposal-card-proposal-1");
    expect(card).toHaveClass("cursor-pointer");
  });

  it("does not add cursor-pointer when onViewDetail not provided", () => {
    render(<ProposalCard {...defaultProps} />);
    const card = screen.getByTestId("proposal-card-proposal-1");
    expect(card).not.toHaveClass("cursor-pointer");
  });
});
