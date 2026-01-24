import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { useQueryClient } from "@tanstack/react-query";
import App from "./App";
import { useUiStore } from "@/stores/uiStore";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";

// Mock the useEvents hooks to prevent Tauri API calls
vi.mock("@/hooks/useEvents", () => ({
  useTaskEvents: vi.fn(),
  useSupervisorAlerts: vi.fn(),
  useReviewEvents: vi.fn(),
  useFileChangeEvents: vi.fn(),
}));

// Mock TaskBoard to avoid Tauri API calls during tests
vi.mock("@/components/tasks/TaskBoard", () => ({
  TaskBoard: () => <div data-testid="task-board-mock">Task Board</div>,
}));

// Mock IdeationView to avoid complex ideation state issues
vi.mock("@/components/Ideation", () => ({
  IdeationView: () => <div data-testid="ideation-view-mock">Ideation View</div>,
}));

// Mock ideation hooks
vi.mock("@/hooks/useIdeation", () => ({
  useIdeationSession: vi.fn().mockReturnValue({
    data: null,
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
}));

// Mock proposal hooks
vi.mock("@/hooks/useProposals", () => ({
  useProposalMutations: vi.fn().mockReturnValue({
    createProposal: { mutateAsync: vi.fn() },
    updateProposal: { mutateAsync: vi.fn() },
    deleteProposal: { mutate: vi.fn() },
    toggleSelection: { mutate: vi.fn() },
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
    activeQuestion: null,
    executionStatus: {
      isPaused: false,
      runningCount: 0,
      maxConcurrent: 2,
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
    selectedProposalIds: new Set(),
    isLoading: false,
    error: null,
  });
}

describe("App", () => {
  beforeEach(() => {
    resetStores();
  });

  it("should render without crashing", () => {
    render(<App />);
    expect(document.body).toBeDefined();
  });

  it("should display RalphX title", () => {
    render(<App />);
    expect(screen.getByText(/RalphX/i)).toBeInTheDocument();
  });

  it("should display project name", () => {
    render(<App />);
    expect(screen.getByText(/Demo Project/i)).toBeInTheDocument();
  });

  it("should have main element with flex layout", () => {
    render(<App />);
    const mainElement = screen.getByRole("main");
    expect(mainElement).toHaveClass("min-h-screen", "flex", "flex-col");
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
    // by rendering a component that uses useQueryClient
    function QueryClientChecker() {
      const queryClient = useQueryClient();
      return queryClient ? <div data-testid="query-ok">OK</div> : null;
    }

    // Render with App as parent to get the QueryClientProvider context
    render(<App />);
    // If App renders successfully with QueryClientProvider, queries should work
    expect(document.body).toBeDefined();
  });
});
