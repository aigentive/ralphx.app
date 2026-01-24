/**
 * Tests for useTaskBoard hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { defaultWorkflow } from "@/types/workflow";
import { createMockTask } from "@/test/mock-data";
import { useTaskBoard } from "./hooks";
import type { DragEndEvent } from "@dnd-kit/core";

// Mock Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
    workflows: {
      get: vi.fn(),
    },
  },
}));

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
      vi.mocked(api.tasks.list).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );
      vi.mocked(api.workflows.get).mockImplementation(
        () => new Promise(() => {})
      );

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      expect(result.current.isLoading).toBe(true);
    });

    it("should return isLoading false when data is loaded", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });
    });
  });

  describe("columns computation", () => {
    it("should return 7 columns from default workflow", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.columns).toHaveLength(7);
      });
    });

    it("should filter tasks into correct columns", async () => {
      const tasks = [
        createMockTask({ id: "t1", internalStatus: "backlog" }),
        createMockTask({ id: "t2", internalStatus: "ready" }),
        createMockTask({ id: "t3", internalStatus: "executing" }),
      ];
      vi.mocked(api.tasks.list).mockResolvedValue(tasks);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const { columns } = result.current;
      // Draft and Backlog both map to "backlog"
      const draftColumn = columns.find((c) => c.id === "draft");
      const backlogColumn = columns.find((c) => c.id === "backlog");
      const todoColumn = columns.find((c) => c.id === "todo");
      const inProgressColumn = columns.find((c) => c.id === "in_progress");

      // Task with backlog status goes to both draft and backlog columns
      expect(draftColumn?.tasks).toHaveLength(1);
      expect(backlogColumn?.tasks).toHaveLength(1);
      // Tasks with ready status go to todo and planned
      expect(todoColumn?.tasks).toHaveLength(1);
      expect(inProgressColumn?.tasks).toHaveLength(1);
    });

    it("should sort tasks by priority within columns", async () => {
      const tasks = [
        createMockTask({ id: "t1", internalStatus: "backlog", priority: 2 }),
        createMockTask({ id: "t2", internalStatus: "backlog", priority: 0 }),
        createMockTask({ id: "t3", internalStatus: "backlog", priority: 1 }),
      ];
      vi.mocked(api.tasks.list).mockResolvedValue(tasks);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const backlogColumn = result.current.columns.find(
        (c) => c.id === "backlog"
      );
      // Higher priority (lower number) should come first
      expect(backlogColumn?.tasks[0]?.id).toBe("t2");
      expect(backlogColumn?.tasks[1]?.id).toBe("t3");
      expect(backlogColumn?.tasks[2]?.id).toBe("t1");
    });
  });

  describe("onDragEnd", () => {
    it("should provide onDragEnd callback", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(typeof result.current.onDragEnd).toBe("function");
    });

    it("should not call move when dropped on same column", async () => {
      const tasks = [createMockTask({ id: "t1", internalStatus: "backlog" })];
      vi.mocked(api.tasks.list).mockResolvedValue(tasks);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);
      vi.mocked(api.tasks.move).mockResolvedValue(tasks[0]!);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const dragEvent: DragEndEvent = {
        active: { id: "t1", data: { current: undefined }, rect: { current: { initial: null, translated: null } } },
        over: { id: "backlog", data: { current: undefined }, rect: null as unknown as DOMRect, disabled: false },
        activatorEvent: new Event("pointerdown"),
        collisions: null,
        delta: { x: 0, y: 0 },
      };

      result.current.onDragEnd(dragEvent);

      // Should not call move because column didn't change
      expect(api.tasks.move).not.toHaveBeenCalled();
    });

    it("should call move mutation when dropped on different column", async () => {
      const tasks = [createMockTask({ id: "t1", internalStatus: "backlog" })];
      vi.mocked(api.tasks.list).mockResolvedValue(tasks);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);
      vi.mocked(api.tasks.move).mockResolvedValue({
        ...tasks[0]!,
        internalStatus: "ready",
      });

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      const dragEvent: DragEndEvent = {
        active: { id: "t1", data: { current: undefined }, rect: { current: { initial: null, translated: null } } },
        over: { id: "todo", data: { current: undefined }, rect: null as unknown as DOMRect, disabled: false },
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
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
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
    it("should return error when tasks fetch fails", async () => {
      vi.mocked(api.tasks.list).mockRejectedValue(new Error("Failed to fetch"));
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.error).toBeDefined();
      });
    });

    it("should return error when workflow fetch fails", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockRejectedValue(
        new Error("Workflow not found")
      );

      const { result } = renderHook(
        () => useTaskBoard("project-1", "ralphx-default"),
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(result.current.error).toBeDefined();
      });
    });
  });
});
