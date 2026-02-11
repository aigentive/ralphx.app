import { describe, it, expect, vi, beforeEach } from "vitest";
import { render as rtlRender, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { PlanningView } from "./PlanningView";
import type { IdeationSession, TaskProposal } from "@/types/ideation";

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: () => () => {},
    emit: vi.fn(),
  }),
}));

// Mock for plan store
const mockClearActivePlan = vi.fn();
const mockSetActivePlan = vi.fn().mockResolvedValue(undefined);
const mockActivePlanByProject: Record<string, string | null> = {};

vi.mock("@/hooks/useDependencyGraph", () => ({
  useDependencyGraph: () => ({ data: null, isFetching: false }),
}));

// Mock for useIdeation - will be replaced per-test for reopen tests
const mockReopenMutate = vi.fn();
const mockResetMutate = vi.fn();

vi.mock("@/hooks/useIdeation", () => ({
  useReopenSession: () => ({ mutate: mockReopenMutate, isPending: false }),
  useResetAndReaccept: () => ({ mutate: mockResetMutate, isPending: false }),
}));

vi.mock("@/hooks/useFileDrop", () => ({
  useFileDrop: () => ({ isDragging: false, dropProps: {}, error: null }),
}));

vi.mock("./useIdeationHandlers", () => ({
  useIdeationHandlers: (
    session: IdeationSession | null,
    _proposals: TaskProposal[],
    _onRemoveProposal: (proposalId: string) => void,
    _onReorderProposals: (proposalIds: string[]) => void,
    onArchiveSession: (sessionId: string) => void
  ) => ({
    highlightedProposalIds: new Set<string>(),
    isPlanExpanded: false,
    setIsPlanExpanded: vi.fn(),
    importStatus: null,
    setImportStatus: vi.fn(),
    fileInputRef: { current: null },
    handleArchive: () => {
      if (session) onArchiveSession(session.id);
    },
    handleClearAll: vi.fn(),
    handleReviewSync: vi.fn(),
    handleUndoSync: vi.fn(),
    handleDismissSync: vi.fn(),
    handleImportPlan: vi.fn(),
    handleFileSelected: vi.fn(),
    handleFileDrop: vi.fn(),
  }),
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: ({ headerContent }: { headerContent?: ReactElement }) => (
    <div data-testid="integrated-chat-panel">{headerContent}</div>
  ),
}));

vi.mock("./PlanBrowser", () => ({
  PlanBrowser: () => <div data-testid="plan-browser" />,
}));

vi.mock("./StartSessionPanel", () => ({
  StartSessionPanel: ({ onNewSession }: { onNewSession: () => void }) => (
    <div data-testid="start-session-panel">
      <button onClick={onNewSession}>Start Session</button>
    </div>
  ),
}));

vi.mock("./ProposalsToolbar", () => ({
  ProposalsToolbar: ({ onAcceptPlan }: { onAcceptPlan: (targetColumn: string) => void }) => (
    <button onClick={() => onAcceptPlan("backlog")}>Accept Plan</button>
  ),
}));

vi.mock("./TieredProposalList", () => ({
  TieredProposalList: ({ proposals }: { proposals: TaskProposal[] }) => (
    <div data-testid="tiered-proposal-list">{proposals.length}</div>
  ),
}));

vi.mock("./ProposalsEmptyState", () => ({
  ProposalsEmptyState: () => <div data-testid="proposals-empty-state" />,
}));

vi.mock("./DropZoneOverlay", () => ({
  DropZoneOverlay: () => null,
}));

vi.mock("./AcceptedSessionBanner", () => ({
  AcceptedSessionBanner: () => <div data-testid="accepted-session-banner" />,
}));

vi.mock("./PlanDisplay", () => ({
  PlanDisplay: () => <div data-testid="plan-display" />,
}));

vi.mock("./ProactiveSyncNotification", () => ({
  ProactiveSyncNotificationBanner: () => null,
}));

vi.mock("./ReopenSessionDialog", () => ({
  ReopenSessionDialog: () => null,
}));

vi.mock("@/stores/planStore", () => ({
  usePlanStore: vi.fn((selector: (state: Record<string, unknown>) => unknown) => {
    const state = {
      setActivePlan: mockSetActivePlan,
      clearActivePlan: mockClearActivePlan,
      activePlanByProject: mockActivePlanByProject,
      planCandidates: [],
      isLoading: false,
      error: null,
      loadActivePlan: vi.fn(),
      loadCandidates: vi.fn(),
    };
    return selector ? selector(state) : state;
  }),
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: vi.fn((selector: (state: Record<string, unknown>) => unknown) => {
    const state = {
      activeProjectId: "project-1",
      projects: {},
      setProjects: vi.fn(),
      updateProject: vi.fn(),
      selectProject: vi.fn(),
      addProject: vi.fn(),
      removeProject: vi.fn(),
    };
    return selector ? selector(state) : state;
  }),
}));

vi.mock("@/components/ui/ResizeHandle", () => ({
  CHAT_PANEL_DEFAULT_WIDTH: 420,
  CHAT_PANEL_MIN_WIDTH: 320,
  ResizeHandle: ({ testId }: { testId?: string }) => <div data-testid={testId ?? "resize-handle"} />,
}));

if (!HTMLElement.prototype.scrollTo) {
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    value: vi.fn(),
    writable: true,
  });
}

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
    createdTaskId: null,
    sortOrder: 1,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
];

function render(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return rtlRender(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
}

describe("PlanningView", () => {
  const defaultProps = {
    session: mockSession,
    sessions: [mockSession],
    proposals: mockProposals,
    onNewSession: vi.fn(),
    onSelectSession: vi.fn(),
    onArchiveSession: vi.fn(),
    onEditProposal: vi.fn(),
    onRemoveProposal: vi.fn(),
    onReorderProposals: vi.fn(),
    onApply: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the main view and active-session layout", () => {
    render(<PlanningView {...defaultProps} />);
    expect(screen.getByTestId("ideation-view")).toBeInTheDocument();
    expect(screen.getByTestId("plan-browser")).toBeInTheDocument();
    expect(screen.getByTestId("ideation-header")).toBeInTheDocument();
    expect(screen.getByTestId("proposals-panel")).toBeInTheDocument();
    expect(screen.getByTestId("conversation-panel")).toBeInTheDocument();
  });

  it("shows title and proposal count in the header", () => {
    render(<PlanningView {...defaultProps} />);
    expect(screen.getByText("Authentication Feature")).toBeInTheDocument();
    expect(screen.getByText("2 proposals")).toBeInTheDocument();
  });

  it("calls onArchiveSession when Archive is clicked", async () => {
    const onArchiveSession = vi.fn();
    const user = userEvent.setup();
    render(<PlanningView {...defaultProps} onArchiveSession={onArchiveSession} />);

    await user.click(screen.getByRole("button", { name: /Archive/i }));
    expect(onArchiveSession).toHaveBeenCalledWith("session-1");
  });

  it("calls onApply with all proposal IDs when accepting plan", async () => {
    const onApply = vi.fn();
    const user = userEvent.setup();
    render(<PlanningView {...defaultProps} onApply={onApply} />);

    await user.click(screen.getByRole("button", { name: "Accept Plan" }));

    expect(onApply).toHaveBeenCalledWith({
      sessionId: "session-1",
      proposalIds: ["proposal-1", "proposal-2"],
      targetColumn: "backlog",
      preserveDependencies: true,
    });
  });

  it("shows empty-state component when there are no proposals", () => {
    render(<PlanningView {...defaultProps} proposals={[]} />);
    expect(screen.getByTestId("proposals-empty-state")).toBeInTheDocument();
  });

  it("renders start-session panel when there is no active session", async () => {
    const onNewSession = vi.fn();
    const user = userEvent.setup();

    render(
      <PlanningView
        {...defaultProps}
        session={null}
        sessions={[]}
        onNewSession={onNewSession}
      />
    );

    expect(screen.getByTestId("start-session-panel")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Start Session" }));
    expect(onNewSession).toHaveBeenCalledTimes(1);
  });

  it("hides Archive action for read-only session", () => {
    render(
      <PlanningView
        {...defaultProps}
        session={{ ...mockSession, status: "accepted" }}
      />
    );

    expect(screen.queryByRole("button", { name: /Archive/i })).not.toBeInTheDocument();
  });

  it("sets active plan after accepting proposals", async () => {
    const onApply = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<PlanningView {...defaultProps} onApply={onApply} />);

    await user.click(screen.getByRole("button", { name: "Accept Plan" }));

    // Wait for async operations to complete
    await vi.waitFor(() => {
      expect(onApply).toHaveBeenCalledWith({
        sessionId: "session-1",
        proposalIds: ["proposal-1", "proposal-2"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    // Verify setActivePlan was called after onApply
    await vi.waitFor(() => {
      expect(mockSetActivePlan).toHaveBeenCalledWith("project-1", "session-1", "ideation");
    });
  });

  it("clears active plan when reopening a session that was the active plan", async () => {
    // Setup: session-1 is the active plan
    mockActivePlanByProject["project-1"] = "session-1";
    mockClearActivePlan.mockResolvedValue(undefined);

    const acceptedSession = {
      ...mockSession,
      status: "accepted" as const,
      convertedAt: "2026-01-24T02:00:00Z",
    };

    const user = userEvent.setup();
    render(
      <PlanningView
        {...defaultProps}
        session={acceptedSession}
      />
    );

    // Click the Reopen button
    const reopenButton = screen.getByRole("button", { name: /Reopen/i });
    await user.click(reopenButton);

    // This should trigger the reopen dialog (mocked away)
    // Simulate the mutation succeeding by calling the onSuccess callback
    expect(mockReopenMutate).toHaveBeenCalledWith(
      "session-1",
      expect.objectContaining({
        onSuccess: expect.any(Function),
        onError: expect.any(Function),
      })
    );

    // Extract and call the onSuccess handler
    const onSuccessHandler = mockReopenMutate.mock.calls[0][1].onSuccess;
    await onSuccessHandler();

    // Verify clearActivePlan was called with the correct project ID
    expect(mockClearActivePlan).toHaveBeenCalledWith("project-1");
  });

  it("does not clear active plan when reopening a different session", async () => {
    // Setup: session-2 is the active plan, we're reopening session-1
    mockActivePlanByProject["project-1"] = "session-2";
    mockClearActivePlan.mockClear();

    const acceptedSession = {
      ...mockSession,
      status: "accepted" as const,
      convertedAt: "2026-01-24T02:00:00Z",
    };

    const user = userEvent.setup();
    render(
      <PlanningView
        {...defaultProps}
        session={acceptedSession}
      />
    );

    // Click the Reopen button
    const reopenButton = screen.getByRole("button", { name: /Reopen/i });
    await user.click(reopenButton);

    // Simulate the mutation succeeding
    const onSuccessHandler = mockReopenMutate.mock.calls[0][1].onSuccess;
    await onSuccessHandler();

    // Verify clearActivePlan was NOT called
    expect(mockClearActivePlan).not.toHaveBeenCalled();
  });

  it("does not clear active plan when no active plan is set", async () => {
    // Setup: no active plan
    mockActivePlanByProject["project-1"] = null;
    mockClearActivePlan.mockClear();

    const acceptedSession = {
      ...mockSession,
      status: "accepted" as const,
      convertedAt: "2026-01-24T02:00:00Z",
    };

    const user = userEvent.setup();
    render(
      <PlanningView
        {...defaultProps}
        session={acceptedSession}
      />
    );

    // Click the Reopen button
    const reopenButton = screen.getByRole("button", { name: /Reopen/i });
    await user.click(reopenButton);

    // Simulate the mutation succeeding
    const onSuccessHandler = mockReopenMutate.mock.calls[0][1].onSuccess;
    await onSuccessHandler();

    // Verify clearActivePlan was NOT called
    expect(mockClearActivePlan).not.toHaveBeenCalled();
  });
});
