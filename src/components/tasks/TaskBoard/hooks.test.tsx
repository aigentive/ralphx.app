/**
 * Tests for useTaskBoard hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { createMockTask } from "@/test/mock-data";
import { useTaskBoard } from "./hooks";
import type { DragEndEvent } from "@dnd-kit/core";
import type { InfiniteData } from "@tanstack/react-query";
import type { TaskListResponse } from "@/types/task";

// Mock Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
  },
}));

// Mock workflows API
vi.mock("@/lib/api/workflows", () => ({
  getActiveWorkflowColumns: vi.fn(),
}));

import { getActiveWorkflowColumns } from "@/lib/api/workflows";
import type { WorkflowColumnResponse } from "@/lib/api/workflows";

// Helper to create mock columns matching WorkflowColumnResponse
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
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("useTaskBoard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("should return isLoading true initially", async () => {
      vi.mocked(getActiveWorkflowColumns).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      expect(result.current.isLoading).toBe(true);
    });

    it("should return isLoading false when data is loaded", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
    });
  });

  describe("columns", () => {
    it("should return 5 columns from default workflow", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.columns).toHaveLength(5);
      });
    });

    it("should return columns with correct structure", async () => {
      const mockColumns = createMockColumns();
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(mockColumns);

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.columns).toHaveLength(5);
      });

      // Columns should have id, name, and mapsTo
      const draftColumn = result.current.columns.find((c) => c.id === "draft");
      expect(draftColumn).toBeDefined();
      expect(draftColumn?.name).toBe("Draft");
      expect(draftColumn?.mapsTo).toBe("backlog");
    });

    it("should support custom workflows with different columns", async () => {
      // Custom methodology workflow with different columns
      const customColumns: WorkflowColumnResponse[] = [
        { id: "backlog", name: "Backlog", mapsTo: "backlog" },
        { id: "selected", name: "Selected for Dev", mapsTo: "ready" },
        { id: "in_dev", name: "In Development", mapsTo: "executing" },
        { id: "qa", name: "QA", mapsTo: "pending_review" },
        { id: "done", name: "Done", mapsTo: "approved" },
      ];
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(customColumns);

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.columns).toHaveLength(5);
      });

      // Should have custom column names
      const selectedColumn = result.current.columns.find((c) => c.id === "selected");
      expect(selectedColumn).toBeDefined();
      expect(selectedColumn?.name).toBe("Selected for Dev");
      expect(selectedColumn?.mapsTo).toBe("ready");
    });
  });

  describe("onDragEnd", () => {
    it("should provide onDragEnd callback", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(typeof result.current.onDragEnd).toBe("function");
    });

    it("should not call move when dropped on same column", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());
      const task = createMockTask({ id: "t1", internalStatus: "backlog" });
      vi.mocked(api.tasks.move).mockResolvedValue(task);

      const queryClient = new QueryClient({
        defaultOptions: { queries: { retry: false } },
      });

      // Pre-populate the cache with the task in the draft column
      // Key format: ["tasks", "infinite", projectId, status, includeArchived]
      const cacheKey = ["tasks", "infinite", "project-1", "backlog", false];
      queryClient.setQueryData(cacheKey, {
        pages: [{ tasks: [task], total: 1, hasMore: false, offset: 0 }],
        pageParams: [undefined],
      } as InfiniteData<TaskListResponse>);

      const wrapper = ({ children }: { children: React.ReactNode }) => (
        <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
      );

      const { result } = renderHook(() => useTaskBoard("project-1"), { wrapper });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const dragEvent: DragEndEvent = {
        active: { id: "t1", data: { current: undefined }, rect: { current: { initial: null, translated: null } } },
        over: { id: "draft", data: { current: undefined }, rect: null as unknown as DOMRect, disabled: false },
        activatorEvent: new Event("pointerdown"),
        collisions: null,
        delta: { x: 0, y: 0 },
      };

      result.current.onDragEnd(dragEvent);

      // Should not call move because column didn't change (draft maps to backlog)
      expect(api.tasks.move).not.toHaveBeenCalled();
    });

    it("should call move mutation when dropped on different column", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());
      const task = createMockTask({ id: "t1", internalStatus: "backlog" });
      vi.mocked(api.tasks.move).mockResolvedValue({
        ...task,
        internalStatus: "ready",
      });

      const queryClient = new QueryClient({
        defaultOptions: { queries: { retry: false } },
      });

      // Pre-populate the cache with the task in the draft column
      // Key format: ["tasks", "infinite", projectId, status, includeArchived]
      const cacheKey = ["tasks", "infinite", "project-1", "backlog", false];
      queryClient.setQueryData(cacheKey, {
        pages: [{ tasks: [task], total: 1, hasMore: false, offset: 0 }],
        pageParams: [undefined],
      } as InfiniteData<TaskListResponse>);

      const wrapper = ({ children }: { children: React.ReactNode }) => (
        <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
      );

      const { result } = renderHook(() => useTaskBoard("project-1"), { wrapper });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const dragEvent: DragEndEvent = {
        active: { id: "t1", data: { current: undefined }, rect: { current: { initial: null, translated: null } } },
        over: { id: "ready", data: { current: undefined }, rect: null as unknown as DOMRect, disabled: false },
        activatorEvent: new Event("pointerdown"),
        collisions: null,
        delta: { x: 0, y: 0 },
      };

      result.current.onDragEnd(dragEvent);

      await waitFor(() => {
        expect(api.tasks.move).toHaveBeenCalledWith("t1", "ready");
      });
    });

    it("should not call move when over is null", async () => {
      vi.mocked(getActiveWorkflowColumns).mockResolvedValue(createMockColumns());

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const dragEvent: DragEndEvent = {
        active: { id: "t1", data: { current: undefined }, rect: { current: { initial: null, translated: null } } },
        over: null,
        activatorEvent: new Event("pointerdown"),
        collisions: null,
        delta: { x: 0, y: 0 },
      };

      result.current.onDragEnd(dragEvent);

      expect(api.tasks.move).not.toHaveBeenCalled();
    });
  });

  describe("error handling", () => {
    it("should return error when columns fetch fails", async () => {
      vi.mocked(getActiveWorkflowColumns).mockRejectedValue(
        new Error("Failed to fetch workflow")
      );

      const { result } = renderHook(
        () => useTaskBoard("project-1"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.error).toBeDefined();
      });
    });
  });
});
