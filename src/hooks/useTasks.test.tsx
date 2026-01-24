import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useTasks } from "./useTasks";
import { api } from "@/lib/tauri";
import type { Task } from "@/types/task";

// Mock the tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
    },
  },
}));

// Helper to create a mock task
const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: "task-1",
  projectId: "project-1",
  category: "feature",
  title: "Test Task",
  description: null,
  priority: 0,
  internalStatus: "backlog",
  needsReviewPoint: false,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  startedAt: null,
  completedAt: null,
  ...overrides,
});

describe("useTasks", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch tasks for a project", async () => {
    const mockTasks = [
      createMockTask({ id: "task-1", title: "Task 1" }),
      createMockTask({ id: "task-2", title: "Task 2" }),
    ];
    vi.mocked(api.tasks.list).mockResolvedValue(mockTasks);

    const { result } = renderHook(() => useTasks("project-123"), { wrapper });

    // Initially loading
    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(api.tasks.list).toHaveBeenCalledWith("project-123");
    expect(result.current.data).toEqual(mockTasks);
    expect(result.current.data).toHaveLength(2);
  });

  it("should handle loading state", async () => {
    // Create a promise that we can control
    let resolvePromise: (value: Task[]) => void;
    const pendingPromise = new Promise<Task[]>((resolve) => {
      resolvePromise = resolve;
    });
    vi.mocked(api.tasks.list).mockReturnValue(pendingPromise);

    const { result } = renderHook(() => useTasks("project-1"), { wrapper });

    // Should be loading initially
    expect(result.current.isLoading).toBe(true);
    expect(result.current.data).toBeUndefined();

    // Resolve the promise
    resolvePromise!([createMockTask()]);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isSuccess).toBe(true);
  });

  it("should handle error state", async () => {
    const error = new Error("Failed to fetch tasks");
    vi.mocked(api.tasks.list).mockRejectedValue(error);

    const { result } = renderHook(() => useTasks("project-1"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toBe(error);
  });

  it("should use projectId in queryKey", async () => {
    vi.mocked(api.tasks.list).mockResolvedValue([]);

    const { result: result1 } = renderHook(() => useTasks("project-a"), {
      wrapper,
    });
    const { result: result2 } = renderHook(() => useTasks("project-b"), {
      wrapper,
    });

    await waitFor(() => {
      expect(result1.current.isSuccess).toBe(true);
      expect(result2.current.isSuccess).toBe(true);
    });

    // Both projects should be fetched separately
    expect(api.tasks.list).toHaveBeenCalledWith("project-a");
    expect(api.tasks.list).toHaveBeenCalledWith("project-b");
    expect(api.tasks.list).toHaveBeenCalledTimes(2);
  });

  it("should return empty array when no tasks exist", async () => {
    vi.mocked(api.tasks.list).mockResolvedValue([]);

    const { result } = renderHook(() => useTasks("empty-project"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual([]);
  });

  it("should not refetch on every render", async () => {
    const mockTasks = [createMockTask()];
    vi.mocked(api.tasks.list).mockResolvedValue(mockTasks);

    const { result, rerender } = renderHook(() => useTasks("project-1"), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // Rerender the hook
    rerender();
    rerender();

    // Should still only have called the API once
    expect(api.tasks.list).toHaveBeenCalledTimes(1);
  });
});
