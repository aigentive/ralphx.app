/**
 * TaskBoardWithHeader component tests
 *
 * Tests for integrating WorkflowSelector with TaskBoard:
 * - Header renders with WorkflowSelector
 * - Workflow switching re-renders columns
 * - Task data preserved during workflow switch
 * - Loading states
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskBoardWithHeader } from "./TaskBoardWithHeader";
import { api } from "@/lib/tauri";
import * as workflowsApi from "@/lib/api/workflows";
import { createMockTask } from "@/test/mock-data";
import type { WorkflowResponse } from "@/lib/api/workflows";

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
    workflows: {
      get: vi.fn(),
      list: vi.fn(),
    },
  },
}));

vi.mock("@/lib/api/workflows", () => ({
  getWorkflows: vi.fn(),
  getWorkflow: vi.fn(),
}));

const mockWorkflows: WorkflowResponse[] = [
  {
    id: "default-workflow",
    name: "Default Workflow",
    description: null,
    columns: [
      { id: "backlog", name: "Backlog", maps_to: "backlog", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
      { id: "in_progress", name: "In Progress", maps_to: "executing", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
      { id: "done", name: "Done", maps_to: "approved", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    ],
    is_default: true,
    worker_profile: null,
    reviewer_profile: null,
  },
  {
    id: "custom-workflow",
    name: "Custom Workflow",
    description: "A custom workflow",
    columns: [
      { id: "todo", name: "To Do", maps_to: "ready", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
      { id: "doing", name: "Doing", maps_to: "executing", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
      { id: "review", name: "Review", maps_to: "pending_review", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
      { id: "complete", name: "Complete", maps_to: "approved", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    ],
    is_default: false,
    worker_profile: null,
    reviewer_profile: null,
  },
];

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("TaskBoardWithHeader", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(mockWorkflows);
    vi.mocked(api.workflows.get).mockResolvedValue(mockWorkflows[0]);
    vi.mocked(api.tasks.list).mockResolvedValue([]);
  });

  // ==========================================================================
  // Header Rendering
  // ==========================================================================

  describe("header rendering", () => {
    it("renders component with testid", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("task-board-with-header")).toBeInTheDocument();
      });
    });

    it("renders workflow selector in header", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("workflow-selector")).toBeInTheDocument();
      });
    });

    it("shows current workflow name", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Default Workflow");
      });
    });

    it("shows default badge for default workflow", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("default-badge")).toBeInTheDocument();
      });
    });
  });

  // ==========================================================================
  // Workflow Switching
  // ==========================================================================

  describe("workflow switching", () => {
    it("lists available workflows in dropdown", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for component and data to load
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Default Workflow");
      });

      // Click to open dropdown
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      // Check dropdown opened
      expect(screen.getByTestId("workflow-dropdown")).toBeInTheDocument();
      const items = screen.getAllByTestId("workflow-item");
      expect(items).toHaveLength(2);
    });

    it("switches workflow when item selected", async () => {
      vi.mocked(api.workflows.get).mockImplementation(async (id) => {
        return mockWorkflows.find((w) => w.id === id) || null;
      });

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Default Workflow");
      });

      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      fireEvent.click(items[1]); // Select custom workflow

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Custom Workflow");
      });
    });

    it("re-renders columns when workflow changes", async () => {
      vi.mocked(api.workflows.get).mockImplementation(async (id) => {
        return mockWorkflows.find((w) => w.id === id) || null;
      });

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Initial workflow has 3 columns
      await waitFor(() => {
        expect(screen.getByTestId("column-backlog")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_progress")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });

      // Switch to custom workflow
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      fireEvent.click(items[1]);

      // Custom workflow has 4 different columns
      await waitFor(() => {
        expect(screen.getByTestId("column-todo")).toBeInTheDocument();
        expect(screen.getByTestId("column-doing")).toBeInTheDocument();
        expect(screen.getByTestId("column-review")).toBeInTheDocument();
        expect(screen.getByTestId("column-complete")).toBeInTheDocument();
      });
    });
  });

  // ==========================================================================
  // Task Data Preservation
  // ==========================================================================

  describe("task data preservation", () => {
    it("does not refetch tasks when workflow switches", async () => {
      // Setup tasks
      vi.mocked(api.tasks.list).mockResolvedValue([
        createMockTask({ id: "t1", title: "Task One", internalStatus: "executing" }),
      ]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for initial load
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Default Workflow");
      });

      // Record call count after initial load
      const initialCallCount = vi.mocked(api.tasks.list).mock.calls.length;

      // Switch workflow
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      fireEvent.click(items[1]);

      // Verify workflow switched
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Custom Workflow");
      });

      // Task list should not have been re-fetched (same project, same query)
      expect(vi.mocked(api.tasks.list).mock.calls.length).toBe(initialCallCount);
    });
  });

  // ==========================================================================
  // Loading States
  // ==========================================================================

  describe("loading states", () => {
    it("shows loading state while workflows loading", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockImplementation(() => new Promise(() => {}));

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      expect(screen.getByTestId("task-board-with-header")).toBeInTheDocument();
      expect(screen.getByTestId("loading-indicator")).toBeInTheDocument();
    });
  });
});
