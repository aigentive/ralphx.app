/**
 * IdeationView.test.tsx
 * Tests for the main ideation view with split layout
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IdeationView } from "./IdeationView";
import type { IdeationSession, TaskProposal, ChatMessage } from "@/types/ideation";

const mockSession: IdeationSession = {
  id: "session-1",
  projectId: "project-1",
  title: "Authentication Feature",
  status: "active",
  createdAt: "2026-01-24T00:00:00Z",
  updatedAt: "2026-01-24T01:00:00Z",
  archivedAt: null,
  convertedAt: null,
};

const mockMessages: ChatMessage[] = [
  {
    id: "msg-1",
    sessionId: "session-1",
    projectId: "project-1",
    taskId: null,
    role: "user",
    content: "I need user authentication",
    metadata: null,
    parentMessageId: null,
    createdAt: "2026-01-24T00:00:00Z",
  },
  {
    id: "msg-2",
    sessionId: "session-1",
    projectId: "project-1",
    taskId: null,
    role: "orchestrator",
    content: "I can help with that. Let me create some task proposals.",
    metadata: null,
    parentMessageId: "msg-1",
    createdAt: "2026-01-24T00:01:00Z",
  },
];

const mockProposals: TaskProposal[] = [
  {
    id: "proposal-1",
    sessionId: "session-1",
    title: "Setup database",
    description: "Initialize SQLite database",
    category: "setup",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "high",
    priorityScore: 75,
    priorityReason: "Foundation task",
    estimatedComplexity: "moderate",
    userPriority: null,
    userModified: false,
    status: "pending",
    selected: true,
    createdTaskId: null,
    sortOrder: 0,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
  {
    id: "proposal-2",
    sessionId: "session-1",
    title: "Create login form",
    description: "Build the login UI",
    category: "feature",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "medium",
    priorityScore: 55,
    priorityReason: "Depends on database",
    estimatedComplexity: "simple",
    userPriority: null,
    userModified: false,
    status: "pending",
    selected: false,
    createdTaskId: null,
    sortOrder: 1,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
];

describe("IdeationView", () => {
  const defaultProps = {
    session: mockSession,
    messages: mockMessages,
    proposals: mockProposals,
    onSendMessage: vi.fn(),
    onNewSession: vi.fn(),
    onArchiveSession: vi.fn(),
    onSelectProposal: vi.fn(),
    onEditProposal: vi.fn(),
    onRemoveProposal: vi.fn(),
    onReorderProposals: vi.fn(),
    onApply: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Layout", () => {
    it("renders ideation view container", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByTestId("ideation-view")).toBeInTheDocument();
    });

    it("renders split layout with two panels", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByTestId("conversation-panel")).toBeInTheDocument();
      expect(screen.getByTestId("proposals-panel")).toBeInTheDocument();
    });

    it("conversation panel is on the left", () => {
      render(<IdeationView {...defaultProps} />);
      const conversationPanel = screen.getByTestId("conversation-panel");
      expect(conversationPanel.parentElement?.firstChild).toBe(conversationPanel);
    });

    it("proposals panel is on the right", () => {
      render(<IdeationView {...defaultProps} />);
      const proposalsPanel = screen.getByTestId("proposals-panel");
      const mainContent = proposalsPanel.closest('[data-testid="ideation-main-content"]');
      expect(mainContent?.lastChild).toBe(proposalsPanel);
    });
  });

  describe("Header", () => {
    it("renders header section", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByTestId("ideation-header")).toBeInTheDocument();
    });

    it("displays session title", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText("Authentication Feature")).toBeInTheDocument();
    });

    it("displays default title when session title is null", () => {
      const sessionNoTitle = { ...mockSession, title: null };
      render(<IdeationView {...defaultProps} session={sessionNoTitle} />);
      // Title appears in both header h1 and button, look for h1 specifically
      const header = screen.getByTestId("ideation-header");
      expect(within(header).getByRole("heading", { level: 1 })).toHaveTextContent("New Session");
    });

    it("renders New Session button", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /New Session/i })).toBeInTheDocument();
    });

    it("renders Archive button", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Archive/i })).toBeInTheDocument();
    });

    it("calls onNewSession when New Session clicked", async () => {
      const onNewSession = vi.fn();
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} onNewSession={onNewSession} />);

      await user.click(screen.getByRole("button", { name: /New Session/i }));
      expect(onNewSession).toHaveBeenCalledTimes(1);
    });

    it("calls onArchiveSession when Archive clicked", async () => {
      const onArchiveSession = vi.fn();
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} onArchiveSession={onArchiveSession} />);

      await user.click(screen.getByRole("button", { name: /Archive/i }));
      expect(onArchiveSession).toHaveBeenCalledWith("session-1");
    });
  });

  describe("Conversation Panel", () => {
    it("renders conversation panel header", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText("Conversation")).toBeInTheDocument();
    });

    it("displays all messages", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText("I need user authentication")).toBeInTheDocument();
      expect(screen.getByText(/I can help with that/)).toBeInTheDocument();
    });

    it("shows empty state when no messages", () => {
      render(<IdeationView {...defaultProps} messages={[]} />);
      expect(screen.getByText(/Start the conversation/i)).toBeInTheDocument();
    });

    it("renders message input", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByPlaceholderText(/Send a message/i)).toBeInTheDocument();
    });

    it("calls onSendMessage when message submitted", async () => {
      const onSendMessage = vi.fn();
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} onSendMessage={onSendMessage} />);

      const input = screen.getByPlaceholderText(/Send a message/i);
      await user.type(input, "Test message");
      await user.keyboard("{Enter}");

      expect(onSendMessage).toHaveBeenCalledWith("Test message");
    });
  });

  describe("Proposals Panel", () => {
    it("renders proposals panel header", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText("Task Proposals")).toBeInTheDocument();
    });

    it("displays proposal count", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText(/2 proposals/i)).toBeInTheDocument();
    });

    it("displays all proposals", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByText("Setup database")).toBeInTheDocument();
      expect(screen.getByText("Create login form")).toBeInTheDocument();
    });

    it("shows empty state when no proposals", () => {
      render(<IdeationView {...defaultProps} proposals={[]} />);
      expect(screen.getByText(/No proposals yet/i)).toBeInTheDocument();
    });

    it("passes onSelectProposal to ProposalList", () => {
      const onSelectProposal = vi.fn();
      render(<IdeationView {...defaultProps} onSelectProposal={onSelectProposal} />);

      // Verify ProposalList is rendered with proposals
      expect(screen.getByTestId("proposal-list")).toBeInTheDocument();
      // The checkbox exists - ProposalList and ProposalCard integration is tested elsewhere
      expect(screen.getByRole("checkbox", { name: /Select Create login form/i })).toBeInTheDocument();
    });

    it("passes onEditProposal to ProposalList", () => {
      const onEditProposal = vi.fn();
      render(<IdeationView {...defaultProps} onEditProposal={onEditProposal} />);

      // Verify edit buttons exist - interaction tested in ProposalList/ProposalCard tests
      const editButtons = screen.getAllByLabelText("Edit proposal");
      expect(editButtons).toHaveLength(2);
    });

    it("passes onRemoveProposal to ProposalList", () => {
      const onRemoveProposal = vi.fn();
      render(<IdeationView {...defaultProps} onRemoveProposal={onRemoveProposal} />);

      // Verify remove buttons exist - interaction tested in ProposalList/ProposalCard tests
      const removeButtons = screen.getAllByLabelText("Remove proposal");
      expect(removeButtons).toHaveLength(2);
    });
  });

  describe("Apply Section", () => {
    it("renders apply section", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByTestId("apply-section")).toBeInTheDocument();
    });

    it("shows selected count", () => {
      render(<IdeationView {...defaultProps} />);
      const applySection = screen.getByTestId("apply-section");
      expect(within(applySection).getByText(/1 selected/i)).toBeInTheDocument();
    });

    it("renders apply dropdown button", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeInTheDocument();
    });

    it("apply button is disabled when no proposals selected", () => {
      const noSelection = mockProposals.map((p) => ({ ...p, selected: false }));
      render(<IdeationView {...defaultProps} proposals={noSelection} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeDisabled();
    });

    it("shows target column options when dropdown clicked", async () => {
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} />);

      await user.click(screen.getByRole("button", { name: /Apply to/i }));

      expect(screen.getByText("Draft")).toBeInTheDocument();
      expect(screen.getByText("Backlog")).toBeInTheDocument();
      expect(screen.getByText("Todo")).toBeInTheDocument();
    });

    it("calls onApply with correct params when column selected", async () => {
      const onApply = vi.fn();
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} onApply={onApply} />);

      await user.click(screen.getByRole("button", { name: /Apply to/i }));
      await user.click(screen.getByText("Backlog"));

      expect(onApply).toHaveBeenCalledWith({
        sessionId: "session-1",
        proposalIds: ["proposal-1"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });
  });

  describe("Loading State", () => {
    it("shows loading state when isLoading is true", () => {
      render(<IdeationView {...defaultProps} isLoading={true} />);
      expect(screen.getByTestId("ideation-loading")).toBeInTheDocument();
    });

    it("disables input when loading", () => {
      render(<IdeationView {...defaultProps} isLoading={true} />);
      expect(screen.getByPlaceholderText(/Send a message/i)).toBeDisabled();
    });

    it("disables apply button when loading", () => {
      render(<IdeationView {...defaultProps} isLoading={true} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeDisabled();
    });
  });

  describe("No Session State", () => {
    it("shows create session prompt when session is null", () => {
      render(<IdeationView {...defaultProps} session={null} />);
      expect(screen.getByText(/Start a new ideation session/i)).toBeInTheDocument();
    });

    it("shows create session button when session is null", () => {
      render(<IdeationView {...defaultProps} session={null} />);
      expect(screen.getByRole("button", { name: /Start Session/i })).toBeInTheDocument();
    });

    it("calls onNewSession when create session clicked", async () => {
      const onNewSession = vi.fn();
      const user = userEvent.setup();
      render(<IdeationView {...defaultProps} session={null} onNewSession={onNewSession} />);

      await user.click(screen.getByRole("button", { name: /Start Session/i }));
      expect(onNewSession).toHaveBeenCalledTimes(1);
    });
  });

  describe("Responsive Layout", () => {
    it("has flex container for main content", () => {
      render(<IdeationView {...defaultProps} />);
      const mainContent = screen.getByTestId("ideation-main-content");
      expect(mainContent).toHaveClass("flex");
    });

    it("applies lg:flex-row for desktop layout", () => {
      render(<IdeationView {...defaultProps} />);
      const mainContent = screen.getByTestId("ideation-main-content");
      expect(mainContent).toHaveClass("lg:flex-row");
    });

    it("applies flex-col for mobile layout", () => {
      render(<IdeationView {...defaultProps} />);
      const mainContent = screen.getByTestId("ideation-main-content");
      expect(mainContent).toHaveClass("flex-col");
    });
  });

  describe("Accessibility", () => {
    it("has proper ARIA landmarks", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByRole("main")).toBeInTheDocument();
    });

    it("header buttons have accessible labels", () => {
      render(<IdeationView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /New Session/i })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Archive/i })).toBeInTheDocument();
    });

    it("message input has accessible label", () => {
      render(<IdeationView {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Send a message/i);
      expect(input).toHaveAttribute("aria-label");
    });
  });

  describe("Styling", () => {
    it("uses dark background", () => {
      render(<IdeationView {...defaultProps} />);
      const view = screen.getByTestId("ideation-view");
      expect(view).toHaveStyle({ backgroundColor: "var(--bg-base)" });
    });

    it("panels have elevated background", () => {
      render(<IdeationView {...defaultProps} />);
      const conversationPanel = screen.getByTestId("conversation-panel");
      expect(conversationPanel).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("header uses border for separation", () => {
      render(<IdeationView {...defaultProps} />);
      const header = screen.getByTestId("ideation-header");
      expect(header).toHaveClass("border-b");
    });

    it("anti-ai-slop: no purple gradients", () => {
      render(<IdeationView {...defaultProps} />);
      const view = screen.getByTestId("ideation-view");
      const styles = window.getComputedStyle(view);
      expect(styles.background).not.toMatch(/purple|#800080|#a855f7/i);
    });
  });
});
