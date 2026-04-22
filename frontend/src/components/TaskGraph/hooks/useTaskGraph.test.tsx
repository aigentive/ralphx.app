/**
 * Tests for useTaskGraph hook
 *
 * Covers:
 * - Guard: disabled when executionPlanId is null (no plan selected or loading gap)
 * - Enabled when executionPlanId is truthy
 * - Debounced task:updated invalidation
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useTaskGraph } from "./useTaskGraph";
import type { TaskDependencyGraphResponse } from "@/api/task-graph";

// Mock the task graph API
const mockGetDependencyGraph = vi.fn();
vi.mock("@/api/task-graph", () => ({
  taskGraphApi: {
    getDependencyGraph: (...args: unknown[]) => mockGetDependencyGraph(...args),
  },
}));

// Mock EventProvider — capture the task:updated subscriber for test control
type Callback = () => void;
const subscribersByEvent: Record<string, Callback[]> = {};
const mockSubscribe = vi.fn((event: string, cb: Callback) => {
  if (!subscribersByEvent[event]) subscribersByEvent[event] = [];
  subscribersByEvent[event].push(cb);
  return () => {
    subscribersByEvent[event] = subscribersByEvent[event].filter((fn) => fn !== cb);
  };
});
vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
    emit: vi.fn(),
  }),
}));

function emitEvent(event: string) {
  (subscribersByEvent[event] ?? []).forEach((cb) => cb());
}

const mockGraphResponse: TaskDependencyGraphResponse = {
  nodes: [],
  edges: [],
  criticalPath: [],
  planGroups: [],
};

describe("useTaskGraph", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    vi.clearAllMocks();
    Object.keys(subscribersByEvent).forEach((k) => delete subscribersByEvent[k]);
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  describe("executionPlanId guard", () => {
    it("should be disabled when executionPlanId is null", async () => {
      renderHook(() => useTaskGraph("project-1", false, null), { wrapper });

      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(mockGetDependencyGraph).not.toHaveBeenCalled();
    });

    it("should be enabled when executionPlanId is set", async () => {
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);

      const { result } = renderHook(
        () => useTaskGraph("project-1", false, "exec-plan-xyz"),
        { wrapper }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(mockGetDependencyGraph).toHaveBeenCalledWith(
        "project-1",
        false,
        "exec-plan-xyz",
        null
      );
    });

    it("should be enabled for a session-scoped graph while executionPlanId is unresolved", async () => {
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);
      usePlanStore.setState({
        activePlanByProject: { "project-1": "session-abc" },
      });

      const { result } = renderHook(
        () => useTaskGraph("project-1", false, null, "session-abc"),
        { wrapper }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(mockGetDependencyGraph).toHaveBeenCalledWith(
        "project-1",
        false,
        null,
        "session-abc"
      );
    });

    it("should be disabled when no active plan exists and executionPlanId is null", async () => {
      // No active plan in store, executionPlanId not resolved — query must stay idle.

      renderHook(() => useTaskGraph("project-1", false, null), { wrapper });

      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(mockGetDependencyGraph).not.toHaveBeenCalled();
    });

    it("should enable query after executionPlanId resolves from null", async () => {
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);

      const { result, rerender } = renderHook(
        ({ execId }: { execId: string | null }) =>
          useTaskGraph("project-1", false, execId),
        { wrapper, initialProps: { execId: null } }
      );

      // Initially disabled
      await new Promise((resolve) => setTimeout(resolve, 50));
      expect(mockGetDependencyGraph).not.toHaveBeenCalled();

      // executionPlanId resolves
      rerender({ execId: "exec-plan-xyz" });
      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(mockGetDependencyGraph).toHaveBeenCalledWith(
        "project-1",
        false,
        "exec-plan-xyz",
        null
      );
    });
  });

  describe("debounced task:updated invalidation", () => {
    it("should debounce multiple rapid task:updated events into one invalidation", () => {
      vi.useFakeTimers();
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);

      // renderHook flushes effects synchronously, so the subscriber is registered
      renderHook(() => useTaskGraph("project-1", false, null), { wrapper });

      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      // Fire 4 rapid task:updated events — only 1 invalidation should result
      emitEvent("task:updated");
      emitEvent("task:updated");
      emitEvent("task:updated");
      emitEvent("task:updated");

      // Debounce timer pending — no invalidation yet
      expect(invalidateSpy).not.toHaveBeenCalled();

      // Advance past debounce threshold
      vi.advanceTimersByTime(500);

      expect(invalidateSpy).toHaveBeenCalledTimes(1);

      vi.useRealTimers();
    });

    it("should not fire invalidation before debounce threshold elapses", () => {
      vi.useFakeTimers();
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);

      renderHook(() => useTaskGraph("project-1", false, null), { wrapper });

      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      emitEvent("task:updated");

      // 499ms — still pending
      vi.advanceTimersByTime(499);
      expect(invalidateSpy).not.toHaveBeenCalled();

      // One more ms — fires
      vi.advanceTimersByTime(1);
      expect(invalidateSpy).toHaveBeenCalledTimes(1);

      vi.useRealTimers();
    });

    it("should cancel pending debounce timer on unmount", () => {
      vi.useFakeTimers();
      mockGetDependencyGraph.mockResolvedValue(mockGraphResponse);

      const { unmount } = renderHook(
        () => useTaskGraph("project-1", false, null),
        { wrapper }
      );

      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      emitEvent("task:updated");

      // Unmount before debounce fires
      unmount();

      vi.advanceTimersByTime(500);

      // Cleanup cancelled the timer — no invalidation should have fired
      expect(invalidateSpy).not.toHaveBeenCalled();

      vi.useRealTimers();
    });
  });
});
