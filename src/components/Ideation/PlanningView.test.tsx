/**
 * PlanningView.test.tsx
 * Tests for the premium planning view with split layout
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanningView } from "./PlanningView";
import type { IdeationSession, TaskProposal, ChatMessage } from "@/types/ideation";

const mockSession: IdeationSession = {
  id: "session-1",
  projectId: "project-1",
  title: "Authentication Feature",
  status: "active",
  planArtifactId: null,
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

describe("PlanningView", () => {
  const defaultProps = {
    session: mockSession,
    sessions: [mockSession],
    messages: mockMessages,
    proposals: mockProposals,
    onSendMessage: vi.fn(),
    onNewSession: vi.fn(),
    onSelectSession: vi.fn(),
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
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByTestId("ideation-view")).toBeInTheDocument();
    });

    it("renders split layout with two panels", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByTestId("conversation-panel")).toBeInTheDocument();
      expect(screen.getByTestId("proposals-panel")).toBeInTheDocument();
    });

    it("renders resize handle between panels", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByTestId("resize-handle")).toBeInTheDocument();
    });

    it("conversation panel has minimum width constraint", () => {
      render(<PlanningView {...defaultProps} />);
      const conversationPanel = screen.getByTestId("conversation-panel");
      expect(conversationPanel).toHaveStyle({ minWidth: "320px" });
    });

    it("proposals panel has minimum width constraint", () => {
      render(<PlanningView {...defaultProps} />);
      const proposalsPanel = screen.getByTestId("proposals-panel");
      expect(proposalsPanel).toHaveStyle({ minWidth: "320px" });
    });
  });

  describe("Header", () => {
    it("renders header section", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByTestId("ideation-header")).toBeInTheDocument();
    });

    it("displays session title", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("Authentication Feature")).toBeInTheDocument();
    });

    it("displays default title when session title is null", () => {
      const sessionNoTitle = { ...mockSession, title: null };
      render(<PlanningView {...defaultProps} session={sessionNoTitle} />);
      const header = screen.getByTestId("ideation-header");
      expect(within(header).getByRole("heading", { level: 1 })).toHaveTextContent("New Session");
    });

    it("renders New Session button with icon", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /New Session/i })).toBeInTheDocument();
    });

    it("renders Archive button with icon", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Archive/i })).toBeInTheDocument();
    });

    it("calls onNewSession when New Session clicked", async () => {
      const onNewSession = vi.fn();
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} onNewSession={onNewSession} />);

      await user.click(screen.getByRole("button", { name: /New Session/i }));
      expect(onNewSession).toHaveBeenCalledTimes(1);
    });

    it("calls onArchiveSession when Archive clicked", async () => {
      const onArchiveSession = vi.fn();
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} onArchiveSession={onArchiveSession} />);

      await user.click(screen.getByRole("button", { name: /Archive/i }));
      expect(onArchiveSession).toHaveBeenCalledWith("session-1");
    });

    it("header has glass effect styling", () => {
      render(<PlanningView {...defaultProps} />);
      const header = screen.getByTestId("ideation-header");
      expect(header).toHaveClass("backdrop-blur-md");
    });
  });

  describe("Conversation Panel", () => {
    it("renders conversation panel header with icon", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("Conversation")).toBeInTheDocument();
    });

    it("displays all messages", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("I need user authentication")).toBeInTheDocument();
      expect(screen.getByText(/I can help with that/)).toBeInTheDocument();
    });

    it("shows empty state when no messages", () => {
      render(<PlanningView {...defaultProps} messages={[]} />);
      expect(screen.getByText(/Start the conversation/i)).toBeInTheDocument();
    });

    it("empty state shows Lucide icon", () => {
      render(<PlanningView {...defaultProps} messages={[]} />);
      // MessageSquareText icon should be present in empty state
      const conversationPanel = screen.getByTestId("conversation-panel");
      expect(conversationPanel.querySelector("svg")).toBeInTheDocument();
    });

    it("renders message input", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByPlaceholderText(/Send a message/i)).toBeInTheDocument();
    });

    it("calls onSendMessage when message submitted", async () => {
      const onSendMessage = vi.fn();
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} onSendMessage={onSendMessage} />);

      const input = screen.getByPlaceholderText(/Send a message/i);
      await user.type(input, "Test message");
      await user.keyboard("{Enter}");

      expect(onSendMessage).toHaveBeenCalledWith("Test message");
    });

    it("user messages have orange background", () => {
      render(<PlanningView {...defaultProps} />);
      const userMessage = screen.getByTestId("chat-message-msg-1");
      const bubble = userMessage.querySelector("div > div");
      expect(bubble).toHaveClass("bg-[var(--accent-primary)]");
    });

    it("AI messages have elevated background", () => {
      render(<PlanningView {...defaultProps} />);
      const aiMessage = screen.getByTestId("chat-message-msg-2");
      const bubble = aiMessage.querySelector("div > div");
      expect(bubble).toHaveClass("bg-[var(--bg-elevated)]");
    });

    it("shows typing indicator when loading", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      expect(screen.getByTestId("typing-indicator")).toBeInTheDocument();
    });
  });

  describe("Proposals Panel", () => {
    it("renders proposals panel header with icon", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("Task Proposals")).toBeInTheDocument();
    });

    it("displays proposal count badge", () => {
      render(<PlanningView {...defaultProps} />);
      // Badge shows count
      const proposalsPanel = screen.getByTestId("proposals-panel");
      expect(within(proposalsPanel).getByText("2")).toBeInTheDocument();
    });

    it("displays all proposals", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("Setup database")).toBeInTheDocument();
      expect(screen.getByText("Create login form")).toBeInTheDocument();
    });

    it("shows empty state when no proposals", () => {
      render(<PlanningView {...defaultProps} proposals={[]} />);
      expect(screen.getByTestId("proposals-empty-state")).toBeInTheDocument();
      expect(screen.getByText(/No proposals yet/i)).toBeInTheDocument();
    });

    it("empty state shows Lightbulb icon", () => {
      render(<PlanningView {...defaultProps} proposals={[]} />);
      const emptyState = screen.getByTestId("proposals-empty-state");
      expect(emptyState.querySelector("svg")).toBeInTheDocument();
    });

    it("renders toolbar with select/sort actions", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText(/1 of 2 selected/i)).toBeInTheDocument();
    });

    it("proposal cards use shadcn Checkbox", async () => {
      render(<PlanningView {...defaultProps} />);
      const checkboxes = screen.getAllByRole("checkbox");
      expect(checkboxes.length).toBeGreaterThan(0);
    });

    it("calls onSelectProposal when checkbox clicked", async () => {
      const onSelectProposal = vi.fn();
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} onSelectProposal={onSelectProposal} />);

      const checkbox = screen.getByRole("checkbox", { name: /Select Create login form/i });
      await user.click(checkbox);

      expect(onSelectProposal).toHaveBeenCalledWith("proposal-2");
    });

    it("selected proposals have orange border", () => {
      render(<PlanningView {...defaultProps} />);
      const selectedCard = screen.getByTestId("proposal-card-proposal-1");
      expect(selectedCard).toHaveClass("border-[var(--accent-primary)]");
    });

    it("proposal cards show priority badges", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("High")).toBeInTheDocument();
      expect(screen.getByText("Medium")).toBeInTheDocument();
    });

    it("proposal cards show category badges", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByText("setup")).toBeInTheDocument();
      expect(screen.getByText("feature")).toBeInTheDocument();
    });
  });

  describe("Apply Section", () => {
    it("renders apply section", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByTestId("apply-section")).toBeInTheDocument();
    });

    it("shows selected count", () => {
      render(<PlanningView {...defaultProps} />);
      const applySection = screen.getByTestId("apply-section");
      expect(within(applySection).getByText(/1 selected/i)).toBeInTheDocument();
    });

    it("renders apply dropdown button with icon", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeInTheDocument();
    });

    it("apply button is disabled when no proposals selected", () => {
      const noSelection = mockProposals.map((p) => ({ ...p, selected: false }));
      render(<PlanningView {...defaultProps} proposals={noSelection} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeDisabled();
    });

    it("shows target column options when dropdown clicked", async () => {
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} />);

      await user.click(screen.getByRole("button", { name: /Apply to/i }));

      expect(screen.getByText("Draft")).toBeInTheDocument();
      expect(screen.getByText("Backlog")).toBeInTheDocument();
      expect(screen.getByText("Todo")).toBeInTheDocument();
    });

    it("calls onApply with correct params when column selected", async () => {
      const onApply = vi.fn();
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} onApply={onApply} />);

      await user.click(screen.getByRole("button", { name: /Apply to/i }));
      await user.click(screen.getByText("Backlog"));

      expect(onApply).toHaveBeenCalledWith({
        sessionId: "session-1",
        proposalIds: ["proposal-1"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    it("uses shadcn DropdownMenu component", async () => {
      const user = userEvent.setup();
      render(<PlanningView {...defaultProps} />);

      await user.click(screen.getByRole("button", { name: /Apply to/i }));

      // shadcn dropdown menu renders in portal with data attributes
      const menuItems = screen.getAllByRole("menuitem");
      expect(menuItems).toHaveLength(3);
    });
  });

  describe("Loading State", () => {
    it("shows loading overlay when isLoading is true", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      expect(screen.getByTestId("ideation-loading")).toBeInTheDocument();
    });

    it("loading overlay has backdrop blur", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      const loading = screen.getByTestId("ideation-loading");
      expect(loading).toHaveClass("backdrop-blur-sm");
    });

    it("disables input when loading", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      expect(screen.getByPlaceholderText(/Send a message/i)).toBeDisabled();
    });

    it("disables apply button when loading", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      expect(screen.getByRole("button", { name: /Apply to/i })).toBeDisabled();
    });

    it("shows typing indicator in messages when loading", () => {
      render(<PlanningView {...defaultProps} isLoading={true} />);
      expect(screen.getByTestId("typing-indicator")).toBeInTheDocument();
    });
  });

  describe("No Session State", () => {
    describe("with no previous sessions (centered layout)", () => {
      it("shows create session prompt when session is null and no previous sessions", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={[]} />);
        expect(screen.getByText(/Start a new ideation session/i)).toBeInTheDocument();
      });

      it("shows create session button with icon when session is null", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={[]} />);
        expect(screen.getByRole("button", { name: /Start Session/i })).toBeInTheDocument();
      });

      it("calls onNewSession when create session clicked", async () => {
        const onNewSession = vi.fn();
        const user = userEvent.setup();
        render(<PlanningView {...defaultProps} session={null} sessions={[]} onNewSession={onNewSession} />);

        await user.click(screen.getByRole("button", { name: /Start Session/i }));
        expect(onNewSession).toHaveBeenCalledTimes(1);
      });

      it("no session state has radial gradient background", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={[]} />);
        const view = screen.getByTestId("ideation-view");
        const styles = view.getAttribute("style") || "";
        expect(styles).toContain("radial-gradient");
      });
    });

    describe("with previous sessions (split layout with session browser)", () => {
      const previousSessions: IdeationSession[] = [
        {
          id: "session-old-1",
          projectId: "project-1",
          title: "Previous Session 1",
          status: "active",
          planArtifactId: null,
          createdAt: "2026-01-20T00:00:00Z",
          updatedAt: "2026-01-21T00:00:00Z",
          archivedAt: null,
          convertedAt: null,
        },
        {
          id: "session-old-2",
          projectId: "project-1",
          title: "Previous Session 2",
          status: "active",
          planArtifactId: null,
          createdAt: "2026-01-22T00:00:00Z",
          updatedAt: "2026-01-23T00:00:00Z",
          archivedAt: null,
          convertedAt: null,
        },
      ];

      it("shows session browser when there are previous active sessions", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={previousSessions} />);
        expect(screen.getByTestId("session-browser")).toBeInTheDocument();
      });

      it("shows session items in the browser", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={previousSessions} />);
        expect(screen.getByText("Previous Session 1")).toBeInTheDocument();
        expect(screen.getByText("Previous Session 2")).toBeInTheDocument();
      });

      it("calls onSelectSession when a session is clicked", async () => {
        const onSelectSession = vi.fn();
        const user = userEvent.setup();
        render(
          <PlanningView
            {...defaultProps}
            session={null}
            sessions={previousSessions}
            onSelectSession={onSelectSession}
          />
        );

        await user.click(screen.getByTestId("session-item-session-old-1"));
        expect(onSelectSession).toHaveBeenCalledWith("session-old-1");
      });

      it("shows start session panel on the right", () => {
        render(<PlanningView {...defaultProps} session={null} sessions={previousSessions} />);
        expect(screen.getByText(/Start a new ideation session/i)).toBeInTheDocument();
      });

      it("filters out archived sessions from the browser", () => {
        const sessionsWithArchived: IdeationSession[] = [
          ...previousSessions,
          {
            id: "session-archived",
            projectId: "project-1",
            title: "Archived Session",
            status: "archived",
            planArtifactId: null,
            createdAt: "2026-01-18T00:00:00Z",
            updatedAt: "2026-01-19T00:00:00Z",
            archivedAt: "2026-01-19T00:00:00Z",
            convertedAt: null,
          },
        ];
        render(<PlanningView {...defaultProps} session={null} sessions={sessionsWithArchived} />);
        expect(screen.queryByText("Archived Session")).not.toBeInTheDocument();
      });
    });
  });

  describe("Read-only Mode", () => {
    it("passes isReadOnly=false when session status is active", () => {
      render(<PlanningView {...defaultProps} />);
      // ProposalsToolbar should have the Accept Plan button enabled (not read-only)
      const acceptButton = screen.getByRole("button", { name: /Accept Plan/i });
      expect(acceptButton).toBeInTheDocument();
    });

    it("passes isReadOnly=true when session status is accepted", () => {
      const acceptedSession = { ...mockSession, status: "accepted" as const };
      render(<PlanningView {...defaultProps} session={acceptedSession} />);
      // In read-only mode, the Accept Plan button should not be rendered
      expect(screen.queryByRole("button", { name: /Accept Plan/i })).not.toBeInTheDocument();
    });

    it("passes isReadOnly=true when session status is archived", () => {
      const archivedSession = { ...mockSession, status: "archived" as const };
      render(<PlanningView {...defaultProps} session={archivedSession} />);
      // In read-only mode, the Accept Plan button should not be rendered
      expect(screen.queryByRole("button", { name: /Accept Plan/i })).not.toBeInTheDocument();
    });
  });

  describe("Premium Styling", () => {
    it("main view has radial gradient background", () => {
      render(<PlanningView {...defaultProps} />);
      const view = screen.getByTestId("ideation-view");
      const styles = view.getAttribute("style") || "";
      expect(styles).toContain("radial-gradient");
    });

    it("panels have surface background", () => {
      render(<PlanningView {...defaultProps} />);
      const conversationPanel = screen.getByTestId("conversation-panel");
      expect(conversationPanel).toHaveClass("bg-[var(--bg-surface)]");
    });

    it("header uses glass effect", () => {
      render(<PlanningView {...defaultProps} />);
      const header = screen.getByTestId("ideation-header");
      expect(header).toHaveClass("backdrop-blur-md");
    });

    it("resize handle glows orange on hover", () => {
      render(<PlanningView {...defaultProps} />);
      const resizeHandle = screen.getByTestId("resize-handle");
      // The inner div has the hover styling
      const innerDiv = resizeHandle.querySelector("div");
      expect(innerDiv).toHaveClass("group-hover:bg-[var(--accent-primary)]");
    });

    it("anti-ai-slop: uses warm orange accent", () => {
      render(<PlanningView {...defaultProps} />);
      const view = screen.getByTestId("ideation-view");
      const styles = view.getAttribute("style");
      // Check for warm orange in gradient
      expect(styles).toContain("255,107,53");
    });

    it("anti-ai-slop: no purple gradients", () => {
      render(<PlanningView {...defaultProps} />);
      const view = screen.getByTestId("ideation-view");
      const styles = view.getAttribute("style") || "";
      expect(styles).not.toMatch(/purple|#800080|#a855f7/i);
    });
  });

  describe("Accessibility", () => {
    it("has proper ARIA landmarks", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("main")).toBeInTheDocument();
    });

    it("header buttons have accessible labels", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("button", { name: /New Session/i })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Archive/i })).toBeInTheDocument();
    });

    it("proposal checkboxes have accessible labels", () => {
      render(<PlanningView {...defaultProps} />);
      expect(screen.getByRole("checkbox", { name: /Select Setup database/i })).toBeInTheDocument();
      expect(screen.getByRole("checkbox", { name: /Select Create login form/i })).toBeInTheDocument();
    });

    it("toolbar buttons have tooltips", () => {
      render(<PlanningView {...defaultProps} />);
      // Tooltip content is rendered on hover - we verify the buttons exist
      const proposalsPanel = screen.getByTestId("proposals-panel");
      const buttons = within(proposalsPanel).getAllByRole("button");
      expect(buttons.length).toBeGreaterThan(0);
    });
  });
});
