/**
 * Tests for useMergeProgressEvents hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useMergeProgressEvents } from "./useMergeProgressEvents";
import type { MergeProgressEvent } from "@/types/events";

// Mock EventBus — must be a STABLE object reference to avoid infinite
// re-renders (bus is in the useEffect dependency array of the hook)
const mockListeners = new Map<string, (payload: unknown) => void>();

const stableBus = {
  subscribe: (eventName: string, callback: (payload: unknown) => void) => {
    mockListeners.set(eventName, callback);
    return () => {
      mockListeners.delete(eventName);
    };
  },
};

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => stableBus,
}));

function emitEvent(eventName: string, payload: unknown) {
  const listener = mockListeners.get(eventName);
  if (listener) {
    listener(payload);
  }
}

function makeProgressEvent(
  overrides: Partial<MergeProgressEvent> = {}
): MergeProgressEvent {
  return {
    task_id: "task-123",
    phase: "worktree_setup",
    status: "started",
    message: "Setting up worktree",
    timestamp: "2026-02-11T10:00:00Z",
    ...overrides,
  };
}

describe("useMergeProgressEvents", () => {
  beforeEach(() => {
    mockListeners.clear();
  });

  it("subscribes to task:merge_progress on mount", () => {
    renderHook(() => useMergeProgressEvents("task-123"));
    expect(mockListeners.has("task:merge_progress")).toBe(true);
  });

  it("unsubscribes on unmount", () => {
    const { unmount } = renderHook(() => useMergeProgressEvents("task-123"));
    expect(mockListeners.has("task:merge_progress")).toBe(true);

    unmount();
    expect(mockListeners.has("task:merge_progress")).toBe(false);
  });

  it("returns empty array initially", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));
    expect(result.current).toEqual([]);
  });

  it("accumulates progress events for matching task", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ phase: "worktree_setup", status: "started" })
      );
    });

    expect(result.current).toHaveLength(1);
    expect(result.current[0].phase).toBe("worktree_setup");
    expect(result.current[0].status).toBe("started");

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ phase: "programmatic_merge", status: "started" })
      );
    });

    expect(result.current).toHaveLength(2);
    expect(result.current[1].phase).toBe("programmatic_merge");
  });

  it("updates existing phase when same phase event arrives", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({
          phase: "typecheck",
          status: "started",
          message: "Running typecheck",
        })
      );
    });

    expect(result.current).toHaveLength(1);
    expect(result.current[0].status).toBe("started");

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({
          phase: "typecheck",
          status: "passed",
          message: "Typecheck passed",
        })
      );
    });

    // Same length — updated in-place, not appended
    expect(result.current).toHaveLength(1);
    expect(result.current[0].status).toBe("passed");
    expect(result.current[0].message).toBe("Typecheck passed");
  });

  it("ignores events for a different task", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ task_id: "task-999", phase: "lint" })
      );
    });

    expect(result.current).toHaveLength(0);
  });

  it("ignores invalid event payloads", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent("task:merge_progress", { invalid: "data" });
    });

    expect(result.current).toHaveLength(0);
  });

  it("resets phases when taskId changes", () => {
    const { result, rerender } = renderHook(
      ({ taskId }) => useMergeProgressEvents(taskId),
      { initialProps: { taskId: "task-123" } }
    );

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ phase: "worktree_setup", status: "passed" })
      );
    });

    expect(result.current).toHaveLength(1);

    // Change taskId — phases should reset
    rerender({ taskId: "task-456" });

    expect(result.current).toHaveLength(0);
  });

  it("handles full phase sequence: started then passed/failed", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    const phases: Array<{
      phase: MergeProgressEvent["phase"];
      status: MergeProgressEvent["status"];
    }> = [
      { phase: "worktree_setup", status: "started" },
      { phase: "worktree_setup", status: "passed" },
      { phase: "programmatic_merge", status: "started" },
      { phase: "programmatic_merge", status: "passed" },
      { phase: "typecheck", status: "started" },
      { phase: "typecheck", status: "passed" },
      { phase: "lint", status: "started" },
      { phase: "lint", status: "failed" },
    ];

    for (const { phase, status } of phases) {
      act(() => {
        emitEvent(
          "task:merge_progress",
          makeProgressEvent({ phase, status, message: `${phase} ${status}` })
        );
      });
    }

    // 4 distinct phases
    expect(result.current).toHaveLength(4);
    expect(result.current[0].phase).toBe("worktree_setup");
    expect(result.current[0].status).toBe("passed");
    expect(result.current[1].phase).toBe("programmatic_merge");
    expect(result.current[1].status).toBe("passed");
    expect(result.current[2].phase).toBe("typecheck");
    expect(result.current[2].status).toBe("passed");
    expect(result.current[3].phase).toBe("lint");
    expect(result.current[3].status).toBe("failed");
  });
});
