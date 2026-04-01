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

  it("subscribes to task:merge_progress and task:merge_phases on mount", () => {
    renderHook(() => useMergeProgressEvents("task-123"));
    expect(mockListeners.has("task:merge_progress")).toBe(true);
    expect(mockListeners.has("task:merge_phases")).toBe(true);
  });

  it("unsubscribes on unmount", () => {
    const { unmount } = renderHook(() => useMergeProgressEvents("task-123"));
    expect(mockListeners.has("task:merge_progress")).toBe(true);

    unmount();
    expect(mockListeners.has("task:merge_progress")).toBe(false);
    expect(mockListeners.has("task:merge_phases")).toBe(false);
  });

  it("returns empty phases and null phaseList initially", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));
    expect(result.current.phases).toEqual([]);
    expect(result.current.phaseList).toBeNull();
  });

  it("accumulates progress events for matching task", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ phase: "worktree_setup", status: "started" })
      );
    });

    expect(result.current.phases).toHaveLength(1);
    expect(result.current.phases[0].phase).toBe("worktree_setup");
    expect(result.current.phases[0].status).toBe("started");

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ phase: "programmatic_merge", status: "started" })
      );
    });

    expect(result.current.phases).toHaveLength(2);
    expect(result.current.phases[1].phase).toBe("programmatic_merge");
  });

  it("updates existing phase when same phase event arrives", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({
          phase: "npm_run_typecheck",
          status: "started",
          message: "Running typecheck",
        })
      );
    });

    expect(result.current.phases).toHaveLength(1);
    expect(result.current.phases[0].status).toBe("started");

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({
          phase: "npm_run_typecheck",
          status: "passed",
          message: "Typecheck passed",
        })
      );
    });

    // Same length — updated in-place, not appended
    expect(result.current.phases).toHaveLength(1);
    expect(result.current.phases[0].status).toBe("passed");
    expect(result.current.phases[0].message).toBe("Typecheck passed");
  });

  it("ignores events for a different task", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent(
        "task:merge_progress",
        makeProgressEvent({ task_id: "task-999", phase: "npm_run_lint" })
      );
    });

    expect(result.current.phases).toHaveLength(0);
  });

  it("ignores invalid event payloads", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent("task:merge_progress", { invalid: "data" });
    });

    expect(result.current.phases).toHaveLength(0);
  });

  it("resets phases and phaseList when taskId changes", () => {
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

    expect(result.current.phases).toHaveLength(1);

    // Change taskId — phases should reset
    rerender({ taskId: "task-456" });

    expect(result.current.phases).toHaveLength(0);
    expect(result.current.phaseList).toBeNull();
  });

  it("handles full phase sequence: started then passed/failed", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    const phases: Array<{
      phase: string;
      status: MergeProgressEvent["status"];
    }> = [
      { phase: "worktree_setup", status: "started" },
      { phase: "worktree_setup", status: "passed" },
      { phase: "programmatic_merge", status: "started" },
      { phase: "programmatic_merge", status: "passed" },
      { phase: "npm_run_typecheck", status: "started" },
      { phase: "npm_run_typecheck", status: "passed" },
      { phase: "npm_run_lint", status: "started" },
      { phase: "npm_run_lint", status: "failed" },
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
    expect(result.current.phases).toHaveLength(4);
    expect(result.current.phases[0].phase).toBe("worktree_setup");
    expect(result.current.phases[0].status).toBe("passed");
    expect(result.current.phases[1].phase).toBe("programmatic_merge");
    expect(result.current.phases[1].status).toBe("passed");
    expect(result.current.phases[2].phase).toBe("npm_run_typecheck");
    expect(result.current.phases[2].status).toBe("passed");
    expect(result.current.phases[3].phase).toBe("npm_run_lint");
    expect(result.current.phases[3].status).toBe("failed");
  });

  it("captures dynamic phase list from task:merge_phases event", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    expect(result.current.phaseList).toBeNull();

    act(() => {
      emitEvent("task:merge_phases", {
        task_id: "task-123",
        phases: [
          { id: "worktree_setup", label: "Worktree Setup" },
          { id: "programmatic_merge", label: "Merge" },
          { id: "npm_run_typecheck", label: "Type Check" },
          { id: "cargo_test", label: "Test" },
          { id: "finalize", label: "Finalize" },
        ],
      });
    });

    expect(result.current.phaseList).toHaveLength(5);
    expect(result.current.phaseList![0].id).toBe("worktree_setup");
    expect(result.current.phaseList![2].id).toBe("npm_run_typecheck");
    expect(result.current.phaseList![2].label).toBe("Type Check");
  });

  it("ignores phase list for different task", () => {
    const { result } = renderHook(() => useMergeProgressEvents("task-123"));

    act(() => {
      emitEvent("task:merge_phases", {
        task_id: "task-999",
        phases: [{ id: "worktree_setup", label: "Setup" }],
      });
    });

    expect(result.current.phaseList).toBeNull();
  });
});
