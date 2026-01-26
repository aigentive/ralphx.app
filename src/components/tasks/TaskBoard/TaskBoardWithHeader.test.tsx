/**
 * TaskBoardWithHeader component tests
 *
 * Tests for integrating WorkflowSelector with TaskBoard:
 * - Header renders with WorkflowSelector
 * - WorkflowSelector shows available workflows
 * - TaskBoard renders with default columns
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskBoardWithHeader } from "./TaskBoardWithHeader";
import { api } from "@/lib/tauri";
import * as workflowsApi from "@/lib/api/workflows";
import type { WorkflowResponse, WorkflowColumnResponse } from "@/lib/api/workflows";
import type { TaskListResponse } from "@/types/task";
import type { InfiniteData } from "@tanstack/react-query";

// Mock IntersectionObserver
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor() {}
}
window.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
      getArchivedCount: vi.fn(),
      search: vi.fn(),
    },
  },
}));

vi.mock("@/lib/api/workflows", () => ({
  getWorkflows: vi.fn(),
  getWorkflow: vi.fn(),
  getActiveWorkflowColumns: vi.fn(),
}));

// Mock useInfiniteTasksQuery
vi.mock("@/hooks/useInfiniteTasksQuery", async (importOriginal) => {
  const actual = await importOriginal() as Record<string, unknown>;
  return {
    ...actual,
    useInfiniteTasksQuery: vi.fn(),
  };
});

// Mock Tauri events
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

import { useInfiniteTasksQuery } from "@/hooks/useInfiniteTasksQuery";

const mockWorkflows: WorkflowResponse[] = [
  {
    id: "default-workflow",
    name: "Default Workflow",
    description: "Default workflow",
    columns: [
      { id: "backlog", name: "Backlog", mapsTo: "backlog" },
      { id: "in_progress", name: "In Progress", mapsTo: "executing" },
      { id: "done", name: "Done", mapsTo: "approved" },
    ],
    isDefault: true,
    workerProfile: undefined,
    reviewerProfile: undefined,
  },
  {
    id: "custom-workflow",
    name: "Custom Workflow",
    description: "A custom workflow",
    columns: [
      { id: "todo", name: "To Do", mapsTo: "ready" },
      { id: "doing", name: "Doing", mapsTo: "executing" },
      { id: "review", name: "Review", mapsTo: "pending_review" },
      { id: "complete", name: "Complete", mapsTo: "approved" },
    ],
    isDefault: false,
    workerProfile: undefined,
    reviewerProfile: undefined,
  },
];

// Default columns returned by getActiveWorkflowColumns
const defaultColumns: WorkflowColumnResponse[] = [
  { id: "draft", name: "Draft", mapsTo: "backlog" },
  { id: "ready", name: "Ready", mapsTo: "ready" },
  { id: "in_progress", name: "In Progress", mapsTo: "executing" },
  { id: "in_review", name: "In Review", mapsTo: "pending_review" },
  { id: "done", name: "Done", mapsTo: "approved" },
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
    // Mock workflows API
    vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(mockWorkflows);
    vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValue(defaultColumns);
    // Mock tasks API
    vi.mocked(api.tasks.getArchivedCount).mockResolvedValue(0);
    vi.mocked(api.tasks.search).mockResolvedValue([]);
    // Mock infinite query
    vi.mocked(useInfiniteTasksQuery).mockReturnValue({
      data: { pages: [{ tasks: [], total: 0, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
      fetchNextPage: vi.fn(),
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      isError: false,
      error: null,
    } as unknown as ReturnType<typeof useInfiniteTasksQuery>);
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
  // Workflow Dropdown
  // ==========================================================================

  describe("workflow dropdown", () => {
    it("lists available workflows in dropdown", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

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

    it("selects workflow from dropdown", async () => {
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
  });

  // ==========================================================================
  // TaskBoard Integration
  // ==========================================================================

  describe("TaskBoard integration", () => {
    it("renders TaskBoard with columns", async () => {
      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        // TaskBoard renders with columns from getActiveWorkflowColumns()
        expect(screen.getByTestId("task-board")).toBeInTheDocument();
      });
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
