import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { useUiStore } from "@/stores/uiStore";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import { useProjectStore } from "@/stores/projectStore";
import { useExecutionStatus } from "@/hooks/useExecutionControl";
import { useExecutionEvents } from "@/hooks/useExecutionEvents";

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock the useEvents hooks to prevent Tauri API calls
vi.mock("@/hooks/useEvents", () => ({
  useTaskEvents: vi.fn(),
  useProposalEvents: vi.fn(),
  useStepEvents: vi.fn(),
  useSupervisorAlerts: vi.fn(),
  useReviewEvents: vi.fn(),
  useFileChangeEvents: vi.fn(),
  useAgentEvents: vi.fn(),
  useExecutionErrorEvents: vi.fn(),
  useRecoveryPromptEvents: vi.fn(),
}));

// Mock TaskBoard to avoid Tauri API calls during tests
vi.mock("@/components/tasks/TaskBoard", () => ({
  TaskBoard: () => <div data-testid="task-board-mock">Task Board</div>,
}));

// Mock IdeationView to avoid complex ideation state issues
vi.mock("@/components/Ideation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/components/Ideation")>();
  return {
    ...actual,
    IdeationView: ({ footer }: { footer?: React.ReactNode }) => (
      <div data-testid="ideation-view-mock">
        Ideation View
        {footer && <div data-testid="ideation-footer-mock">{footer}</div>}
      </div>
    ),
  };
});

// Mock ExtensibilityView
vi.mock("@/components/ExtensibilityView", () => ({
  ExtensibilityView: () => <div data-testid="extensibility-view-mock">Extensibility View</div>,
}));

// Mock ActivityView
vi.mock("@/components/activity", () => ({
  ActivityView: ({ showHeader }: { showHeader?: boolean }) => (
    <div data-testid="activity-view-mock">Activity View {showHeader && "(with header)"}</div>
  ),
}));

// Mock SettingsView
vi.mock("@/components/settings", () => ({
  SettingsView: () => <div data-testid="settings-view-mock">Settings View</div>,
}));

// Mock ProjectSelector
vi.mock("@/components/projects/ProjectSelector", () => ({
  ProjectSelector: ({ onNewProject }: { onNewProject?: () => void }) => (
    <button
      data-testid="project-selector-mock"
      onClick={onNewProject}
      aria-label="Select project"
    >
      Demo Project
    </button>
  ),
}));

// Mock ProjectCreationWizard
vi.mock("@/components/projects/ProjectCreationWizard", () => ({
  ProjectCreationWizard: () => null,
}));

// Mock ideation hooks
vi.mock("@/hooks/useIdeation", () => ({
  useIdeationSession: vi.fn().mockReturnValue({
    data: null,
    isLoading: false,
  }),
  useIdeationSessions: vi.fn().mockReturnValue({
    data: [],
    isLoading: false,
  }),
  useCreateIdeationSession: vi.fn().mockReturnValue({
    mutateAsync: vi.fn(),
    isPending: false,
  }),
  useArchiveIdeationSession: vi.fn().mockReturnValue({
    mutateAsync: vi.fn(),
    isPending: false,
  }),
  useDeleteIdeationSession: vi.fn().mockReturnValue({
    mutateAsync: vi.fn(),
    isPending: false,
  }),
}));

// Mock proposal hooks
vi.mock("@/hooks/useProposals", () => ({
  useProposalMutations: vi.fn().mockReturnValue({
    createProposal: { mutateAsync: vi.fn() },
    updateProposal: { mutateAsync: vi.fn() },
    deleteProposal: { mutate: vi.fn() },
    reorder: { mutate: vi.fn() },
  }),
}));

// Mock apply proposals hook
vi.mock("@/hooks/useApplyProposals", () => ({
  useApplyProposals: vi.fn().mockReturnValue({
    apply: {
      mutateAsync: vi.fn(),
      isPending: false,
    },
  }),
}));

// Mock execution hooks
vi.mock("@/hooks/useExecutionControl", () => ({
  useExecutionStatus: vi.fn(),
}));

vi.mock("@/hooks/useExecutionEvents", () => ({
  useExecutionEvents: vi.fn(),
}));

// Mock other required hooks
vi.mock("@/hooks/useReviews", () => ({
  reviewKeys: {
    all: ["reviews"],
    pending: () => ["reviews", "pending"],
  },
  usePendingReviews: vi.fn().mockReturnValue({
    data: [],
    isLoading: false,
  }),
  useReviewsByTaskId: vi.fn().mockReturnValue({
    data: [],
    isLoading: false,
  }),
  useTaskStateHistory: vi.fn().mockReturnValue({
    data: [],
    isLoading: false,
  }),
  useTasksAwaitingReview: vi.fn().mockReturnValue({
    totalCount: 0,
    isLoading: false,
  }),
}));

vi.mock("@/hooks/useReviewMutations", () => ({
  useReviewMutations: vi.fn().mockReturnValue({
    isApproving: false,
    isRequestingChanges: false,
  }),
}));

vi.mock("@/hooks/useProjects", () => ({
  useProjects: vi.fn().mockReturnValue({
    data: [],
    isLoading: false,
  }),
  projectKeys: {
    all: ["projects"],
    list: () => ["projects", "list"],
  },
}));

vi.mock("@/hooks/useConfirmation", () => ({
  useConfirmation: vi.fn().mockReturnValue({
    confirm: vi.fn(),
    confirmationDialogProps: {},
    ConfirmationDialog: () => null,
  }),
}));

vi.mock("@/hooks/useAppKeyboardShortcuts", () => ({
  useAppKeyboardShortcuts: vi.fn(),
}));

vi.mock("@/hooks", () => ({
  useNavCompactBreakpoint: vi.fn().mockReturnValue({
    isNavCompact: false,
  }),
}));

// Reset stores before each test
function resetStores() {
  useUiStore.setState({
    sidebarOpen: true,
    reviewsPanelOpen: false,
    currentView: "kanban",
    activeModal: null,
    modalContext: undefined,
    notifications: [],
    loading: {},
    confirmation: null,
    activeQuestions: {},
    answeredQuestions: {},
    executionStatus: {
      isPaused: false,
      runningCount: 0,
      maxConcurrent: 10,
      queuedCount: 0,
      canStartTask: true,
    },
  });

  useChatStore.setState({
    messages: {},
    context: {
      view: "kanban",
      projectId: "demo-project",
    },
    isOpen: false,
    width: 320,
    isLoading: false,
  });

  useIdeationStore.setState({
    sessions: {},
    activeSessionId: null,
    isLoading: false,
    error: null,
  });

  useProposalStore.setState({
    proposals: {},
    isLoading: false,
    error: null,
    lastProposalAddedAt: null,
    lastDependencyRefreshRequestedAt: null,
    lastProposalUpdatedAt: null,
    lastUpdatedProposalId: null,
  });

  useProjectStore.setState({
    activeProjectId: null,
    projects: {},
    isInitialized: false,
  });
}

describe("App", () => {
  beforeEach(() => {
    resetStores();

    // Setup default mock return values for execution hooks
    vi.mocked(useExecutionStatus).mockReturnValue({
      data: {
        isPaused: false,
        runningCount: 0,
        maxConcurrent: 2,
        queuedCount: 0,
        canStartTask: true,
      },
      isPaused: false,
      runningCount: 0,
      queuedCount: 0,
      maxConcurrent: 2,
      globalMaxConcurrent: 20,
      canStartTask: true,
      isLoading: false,
    });

    vi.mocked(useExecutionEvents).mockReturnValue(undefined);
  });

  it("should render without crashing", () => {
    render(<App />);
    expect(document.body).toBeDefined();
  });

  it("should display RalphX title", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: /Ralph/i })).toBeInTheDocument();
  });

  it("should display project selector", () => {
    render(<App />);
    // ProjectSelector is mocked, check for the mock element
    expect(screen.getByTestId("project-selector-mock")).toBeInTheDocument();
  });

  it("should have main element with flex layout", () => {
    render(<App />);
    const mainElement = screen.getByRole("main");
    // h-screen for fixed header layout (header is fixed, needs explicit height)
    expect(mainElement).toHaveClass("h-screen", "flex", "flex-col");
  });

  it("should render header with RalphX branding", () => {
    render(<App />);
    const header = screen.getByRole("banner");
    expect(header).toBeInTheDocument();
    expect(header).toHaveClass("flex", "items-center", "justify-between");
  });

  it("should render TaskBoard component", () => {
    render(<App />);
    expect(screen.getByTestId("task-board-mock")).toBeInTheDocument();
  });

  it("should provide QueryClient context", () => {
    // This test verifies that QueryClientProvider is working
    // If App renders successfully with QueryClientProvider, queries should work
    render(<App />);
    expect(document.body).toBeDefined();
  });

  describe("View Navigation", () => {
    it("should render all navigation buttons", () => {
      render(<App />);
      expect(screen.getByTestId("nav-kanban")).toBeInTheDocument();
      expect(screen.getByTestId("nav-ideation")).toBeInTheDocument();
      expect(screen.getByTestId("nav-extensibility")).toBeInTheDocument();
      expect(screen.getByTestId("nav-activity")).toBeInTheDocument();
      expect(screen.getByTestId("nav-settings")).toBeInTheDocument();
    });

    it("should have navigation buttons rendered as accessible elements", () => {
      render(<App />);
      // All nav buttons should exist and have proper accessible labels
      // (Using shadcn Tooltip which provides keyboard shortcut info on hover)
      const navButtons = [
        { testId: "nav-kanban", label: /Kanban/i },
        { testId: "nav-ideation", label: /Ideation/i },
        { testId: "nav-extensibility", label: /Extensibility/i },
        { testId: "nav-activity", label: /Activity/i },
        { testId: "nav-settings", label: /Settings/i },
      ];
      for (const { testId, label } of navButtons) {
        const btn = screen.getByTestId(testId);
        expect(btn).toBeInTheDocument();
        expect(btn).toHaveTextContent(label);
      }
    });

    it("should start with Kanban view active", () => {
      render(<App />);
      expect(screen.getByTestId("nav-kanban")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("task-board-mock")).toBeInTheDocument();
    });

    it("should switch to Ideation view when clicked", async () => {
      const user = userEvent.setup();
      render(<App />);

      await user.click(screen.getByTestId("nav-ideation"));

      expect(screen.getByTestId("nav-ideation")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("ideation-view-mock")).toBeInTheDocument();
      expect(screen.queryByTestId("task-board-mock")).not.toBeInTheDocument();
    });

    it("should pass footer with ExecutionControlBar to Ideation view", async () => {
      const user = userEvent.setup();
      render(<App />);

      await user.click(screen.getByTestId("nav-ideation"));

      // Verify footer is rendered in the ideation view mock
      expect(screen.getByTestId("ideation-footer-mock")).toBeInTheDocument();
    });

    it("should switch to Extensibility view when clicked", async () => {
      const user = userEvent.setup();
      render(<App />);

      await user.click(screen.getByTestId("nav-extensibility"));

      expect(screen.getByTestId("nav-extensibility")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("extensibility-view-mock")).toBeInTheDocument();
      expect(screen.queryByTestId("task-board-mock")).not.toBeInTheDocument();
    });

    it("should switch to Activity view when clicked", async () => {
      const user = userEvent.setup();
      render(<App />);

      await user.click(screen.getByTestId("nav-activity"));

      expect(screen.getByTestId("nav-activity")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("activity-view-mock")).toBeInTheDocument();
      expect(screen.queryByTestId("task-board-mock")).not.toBeInTheDocument();
    });

    it("should switch to Settings view when clicked", async () => {
      const user = userEvent.setup();
      render(<App />);

      await user.click(screen.getByTestId("nav-settings"));

      expect(screen.getByTestId("nav-settings")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("settings-view-mock")).toBeInTheDocument();
      expect(screen.queryByTestId("task-board-mock")).not.toBeInTheDocument();
    });

    it("should switch views correctly multiple times", async () => {
      const user = userEvent.setup();
      render(<App />);

      // Start on Kanban
      expect(screen.getByTestId("task-board-mock")).toBeInTheDocument();

      // Go to Activity
      await user.click(screen.getByTestId("nav-activity"));
      expect(screen.getByTestId("activity-view-mock")).toBeInTheDocument();

      // Go to Settings
      await user.click(screen.getByTestId("nav-settings"));
      expect(screen.getByTestId("settings-view-mock")).toBeInTheDocument();

      // Go back to Kanban
      await user.click(screen.getByTestId("nav-kanban"));
      expect(screen.getByTestId("task-board-mock")).toBeInTheDocument();
    });

    it("should remove aria-current from previous nav when switching", async () => {
      const user = userEvent.setup();
      render(<App />);

      // Kanban is active initially
      expect(screen.getByTestId("nav-kanban")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("nav-activity")).not.toHaveAttribute("aria-current");

      // Switch to Activity
      await user.click(screen.getByTestId("nav-activity"));

      // Activity is now active, Kanban is not
      expect(screen.getByTestId("nav-activity")).toHaveAttribute("aria-current", "page");
      expect(screen.getByTestId("nav-kanban")).not.toHaveAttribute("aria-current");
    });
  });

  describe("Keyboard Shortcuts", () => {
    it("should switch to Ideation with Cmd+1", () => {
      render(<App />);
      // First switch away from ideation
      useUiStore.setState({ currentView: "activity" });
      render(<App />);

      fireEvent.keyDown(window, { key: "1", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("ideation");
    });

    it("should switch to Graph with Cmd+2", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "2", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("graph");
    });

    it("should switch to Kanban with Cmd+3", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "3", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("kanban");
    });

    it("should switch to Extensibility with Cmd+4", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "4", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("extensibility");
    });

    it("should switch to Activity with Cmd+5", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "5", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("activity");
    });

    it("should switch to Settings with Cmd+6", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "6", metaKey: true });

      expect(useUiStore.getState().currentView).toBe("settings");
    });

    it("should work with Ctrl key (for non-Mac)", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "4", ctrlKey: true });

      expect(useUiStore.getState().currentView).toBe("extensibility");
    });

    it("should not switch views when pressing number without modifier", () => {
      render(<App />);

      fireEvent.keyDown(window, { key: "4" });

      // Should still be on kanban (default)
      expect(useUiStore.getState().currentView).toBe("kanban");
    });
  });

  describe("Execution Status Query Scoping", () => {
    it("should call useExecutionStatus with undefined when no active project", () => {
      // This test verifies Phase 82 requirement: execution status queries are scoped to active project
      // When no project is set, activeProjectId is null, so currentProjectId = ""
      // and "" || undefined = undefined
      render(<App />);

      // useExecutionStatus should be called with undefined (no active project)
      expect(vi.mocked(useExecutionStatus)).toHaveBeenCalledWith(undefined);
    });

    it("should call useExecutionStatus with project ID when project is active", () => {
      // Set up an active project BEFORE rendering
      useProjectStore.setState({ activeProjectId: "test-project-123" });

      render(<App />);

      // When a project is active, useExecutionStatus should be called with that project ID
      expect(vi.mocked(useExecutionStatus)).toHaveBeenCalledWith("test-project-123");
    });
  });
});
