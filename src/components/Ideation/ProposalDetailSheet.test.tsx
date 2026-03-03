import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalDetailSheet } from "./ProposalDetailSheet";
import type { ProposalDetailEnrichment } from "./ProposalDetailSheet";
import type { TaskProposal } from "@/types/ideation";

function createMockProposal(overrides: Partial<TaskProposal> = {}): TaskProposal {
  return {
    id: "proposal-1",
    sessionId: "session-1",
    title: "Implement authentication",
    description: "Add JWT-based auth system with refresh tokens",
    category: "feature",
    steps: ["Create auth service", "Add login endpoint", "Add token refresh"],
    acceptanceCriteria: ["Users can login", "Tokens expire correctly", "Refresh works"],
    suggestedPriority: "high",
    priorityScore: 80,
    priorityReason: "Core security requirement",
    estimatedComplexity: "moderate",
    userPriority: null,
    userModified: false,
    status: "pending",
    createdTaskId: null,
    planArtifactId: null,
    planVersionAtCreation: null,
    sortOrder: 0,
    createdAt: "2026-01-24T10:00:00Z",
    updatedAt: "2026-01-24T10:00:00Z",
    ...overrides,
  };
}

function createMockEnrichment(overrides: Partial<ProposalDetailEnrichment> = {}): ProposalDetailEnrichment {
  return {
    dependsOnDetails: [],
    blocksCount: 0,
    isOnCriticalPath: false,
    ...overrides,
  };
}

describe("ProposalDetailSheet", () => {
  const defaultOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders null when proposal is null", () => {
    const { container } = render(
      <ProposalDetailSheet proposal={null} onClose={defaultOnClose} />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("shows title and description", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Implement authentication")).toBeInTheDocument();
    expect(screen.getByText("Add JWT-based auth system with refresh tokens")).toBeInTheDocument();
  });

  it("shows numbered implementation steps", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Create auth service")).toBeInTheDocument();
    expect(screen.getByText("Add login endpoint")).toBeInTheDocument();
    expect(screen.getByText("Add token refresh")).toBeInTheDocument();
  });

  it("shows acceptance criteria", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Users can login")).toBeInTheDocument();
    expect(screen.getByText("Tokens expire correctly")).toBeInTheDocument();
    expect(screen.getByText("Refresh works")).toBeInTheDocument();
  });

  it("shows metadata chips: priority, category, complexity", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("High")).toBeInTheDocument();
    expect(screen.getByText("feature")).toBeInTheDocument();
    expect(screen.getByText("Moderate")).toBeInTheDocument();
  });

  it("shows priority reason when provided", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText(/"Core security requirement"/)).toBeInTheDocument();
  });

  it("shows critical path badge when on critical path", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        enrichment={createMockEnrichment({ isOnCriticalPath: true })}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Critical Path")).toBeInTheDocument();
  });

  it("does not show critical path badge when not on critical path", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        enrichment={createMockEnrichment({ isOnCriticalPath: false })}
        onClose={defaultOnClose}
      />
    );
    expect(screen.queryByText("Critical Path")).not.toBeInTheDocument();
  });

  it("shows dependencies when provided", () => {
    const enrichment = createMockEnrichment({
      dependsOnDetails: [
        { proposalId: "dep-1", title: "Setup DB", reason: "Schema first" },
        { proposalId: "dep-2", title: "Define Types" },
      ],
    });
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        enrichment={enrichment}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Setup DB")).toBeInTheDocument();
    expect(screen.getByText("Schema first")).toBeInTheDocument();
    expect(screen.getByText("Define Types")).toBeInTheDocument();
  });

  it("shows blocks count when > 0", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        enrichment={createMockEnrichment({ blocksCount: 3 })}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByText("Blocks 3 proposals")).toBeInTheDocument();
  });

  it("shows View Task button when proposal has createdTaskId", () => {
    const onNavigateToTask = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal({ createdTaskId: "task-abc" })}
        onClose={defaultOnClose}
        onNavigateToTask={onNavigateToTask}
      />
    );
    expect(screen.getByTestId("view-task-from-detail")).toBeInTheDocument();
  });

  it("edit button present when not read-only and onEdit provided", () => {
    const onEdit = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        isReadOnly={false}
        onClose={defaultOnClose}
        onEdit={onEdit}
      />
    );
    expect(screen.getByTestId("edit-proposal-button")).toBeInTheDocument();
  });

  it("edit button absent when isReadOnly=true", () => {
    const onEdit = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        isReadOnly={true}
        onClose={defaultOnClose}
        onEdit={onEdit}
      />
    );
    expect(screen.queryByTestId("edit-proposal-button")).not.toBeInTheDocument();
  });

  it("edit button absent when onEdit not provided", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        isReadOnly={false}
        onClose={defaultOnClose}
      />
    );
    expect(screen.queryByTestId("edit-proposal-button")).not.toBeInTheDocument();
  });

  it("calls onClose when X button clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={onClose}
      />
    );
    await user.click(screen.getByTestId("close-sheet-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when backdrop clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={onClose}
      />
    );
    await user.click(screen.getByTestId("proposal-detail-backdrop"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onEdit with proposal id when edit button clicked", async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        isReadOnly={false}
        onClose={defaultOnClose}
        onEdit={onEdit}
      />
    );
    await user.click(screen.getByTestId("edit-proposal-button"));
    expect(onEdit).toHaveBeenCalledWith("proposal-1");
  });

  it("panel is present in DOM when proposal provided", () => {
    render(
      <ProposalDetailSheet
        proposal={createMockProposal()}
        onClose={defaultOnClose}
      />
    );
    expect(screen.getByTestId("proposal-detail-sheet")).toBeInTheDocument();
  });
});
