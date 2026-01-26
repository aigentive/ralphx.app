/**
 * Tests for TaskBoard component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { createMockTask } from "@/test/mock-data";
import { TaskBoard } from "./TaskBoard";
import type { TaskListResponse } from "@/types/task";
import type { InfiniteData } from "@tanstack/react-query";
import type { WorkflowColumnResponse } from "@/lib/api/workflows";

// Mock IntersectionObserver
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor() {}
}
window.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

// Mock Tauri API
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

// Mock workflows API
vi.mock("@/lib/api/workflows", () => ({
  getActiveWorkflowColumns: vi.fn(),
}));

// Mock useInfiniteTasksQuery - keep flattenPages implementation, only mock the hook
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

import { getActiveWorkflowColumns } from "@/lib/api/workflows";
import { useInfiniteTasksQuery } from "@/hooks/useInfiniteTasksQuery";

// Helper to create mock columns
function createMockColumns(): WorkflowColumnResponse[] {
  return [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "ready", name: "Ready", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ];
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("TaskBoard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default mock for archived count
    vi.mocked(api.tasks.getArchivedCount).mockResolvedValue(0);
    // Default mock for search
    vi.mocked(api.tasks.search).mockResolvedValue([]);
    // Default mock for infinite query
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

  describe("loading state", () => {
    it("should show skeleton while loading", async () => {
      vi.mocked(getActiveWorkflowColumns).mockImplementation(() => new Promise(() => {}));

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });
      expect(screen.getByTestId("task-board-skeleton")).toBeInTheDocument();
    });

    it("should hide skeleton when data is loaded", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.queryByTestId("task-board-skeleton")).not.toBeInTheDocument();
      });
    });
  });

  describe("rendering columns", () => {
    it("should render with data-testid", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("task-board")).toBeInTheDocument();
      });
    });

    it("should render 5 columns from default workflow", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
        expect(screen.getByTestId("column-ready")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_progress")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_review")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });
    });

    it("should render tasks in their columns", async () => {
      const tasks = [
        createMockTask({ id: "t1", title: "Task One", internalStatus: "backlog" }),
        createMockTask({ id: "t2", title: "Task Two", internalStatus: "ready" }),
      ];
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      // Mock the infinite query to return tasks based on status
      vi.mocked(useInfiniteTasksQuery).mockImplementation((params) => {
        const tasksForStatus = tasks.filter(t => t.internalStatus === params.status);
        return {
          data: { pages: [{ tasks: tasksForStatus, total: tasksForStatus.length, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
          fetchNextPage: vi.fn(),
          hasNextPage: false,
          isFetchingNextPage: false,
          isLoading: false,
          isError: false,
          error: null,
        } as unknown as ReturnType<typeof useInfiniteTasksQuery>;
      });

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByText("Task One")).toBeInTheDocument();
        expect(screen.getByText("Task Two")).toBeInTheDocument();
      });
    });
  });

  describe("horizontal scrolling", () => {
    it("should have horizontal scroll container", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        const board = screen.getByTestId("task-board");
        expect(board).toHaveClass("overflow-x-auto");
      });
    });
  });

  describe("error handling", () => {
    it("should show error message when fetch fails", async () => {
      vi.mocked(getActiveWorkflowColumns).mockRejectedValue(new Error("Failed to fetch"));

      render(<TaskBoard projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("task-board-error")).toBeInTheDocument();
      });
    });
  });
});
