/**
 * Tests for useExecutionEvents hook
 *
 * Verifies that the hook correctly:
 * - Listens for execution:status_changed events
 * - Listens for execution:queue_changed events
 * - Updates the UI store with event data
 * - Cleans up listeners on unmount
 */

import { describe, it, expect, beforeEach, vi, type Mock } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { listen } from "@tauri-apps/api/event";
import { useExecutionEvents } from "./useExecutionEvents";
import { useUiStore } from "@/stores/uiStore";

// Type for the event callback captured by listen mock
type EventCallback<T> = (event: { payload: T }) => void;

// Event payload types
interface ExecutionStatusEvent {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  reason: string;
  timestamp: string;
}

interface ExecutionQueueEvent {
  queuedCount: number;
  timestamp: string;
}

describe("useExecutionEvents", () => {
  // Track captured event handlers
  let statusChangedHandler: EventCallback<ExecutionStatusEvent> | null = null;
  let queueChangedHandler: EventCallback<ExecutionQueueEvent> | null = null;
  let unlistenStatusMock: Mock;
  let unlistenQueueMock: Mock;

  beforeEach(() => {
    // Reset store to initial state
    useUiStore.setState({
      executionStatus: {
        isPaused: false,
        runningCount: 0,
        maxConcurrent: 2,
        queuedCount: 0,
        canStartTask: true,
      },
    });

    // Reset captured handlers
    statusChangedHandler = null;
    queueChangedHandler = null;

    // Create unlisten mocks
    unlistenStatusMock = vi.fn();
    unlistenQueueMock = vi.fn();

    // Mock listen to capture the event handlers
    (listen as Mock).mockImplementation((eventName: string, callback) => {
      if (eventName === "execution:status_changed") {
        statusChangedHandler = callback as EventCallback<ExecutionStatusEvent>;
        return Promise.resolve(unlistenStatusMock);
      }
      if (eventName === "execution:queue_changed") {
        queueChangedHandler = callback as EventCallback<ExecutionQueueEvent>;
        return Promise.resolve(unlistenQueueMock);
      }
      return Promise.resolve(vi.fn());
    });
  });

  describe("status_changed event", () => {
    it("registers listener for execution:status_changed", () => {
      renderHook(() => useExecutionEvents());

      expect(listen).toHaveBeenCalledWith(
        "execution:status_changed",
        expect.any(Function)
      );
    });

    it("updates store isPaused on status_changed event", async () => {
      renderHook(() => useExecutionEvents());

      // Simulate event
      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: true,
            runningCount: 1,
            maxConcurrent: 2,
            reason: "paused",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.isPaused).toBe(true);
    });

    it("updates store runningCount on status_changed event", async () => {
      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: false,
            runningCount: 2,
            maxConcurrent: 3,
            reason: "task_started",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.runningCount).toBe(2);
    });

    it("updates store maxConcurrent on status_changed event", async () => {
      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: false,
            runningCount: 0,
            maxConcurrent: 5,
            reason: "config_changed",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.maxConcurrent).toBe(5);
    });

    it("calculates canStartTask correctly when not paused and under limit", async () => {
      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: false,
            runningCount: 1,
            maxConcurrent: 2,
            reason: "task_started",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.canStartTask).toBe(true);
    });

    it("calculates canStartTask correctly when paused", async () => {
      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: true,
            runningCount: 0,
            maxConcurrent: 2,
            reason: "paused",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.canStartTask).toBe(false);
    });

    it("calculates canStartTask correctly when at max capacity", async () => {
      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: false,
            runningCount: 2,
            maxConcurrent: 2,
            reason: "task_started",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.canStartTask).toBe(false);
    });

    it("preserves queuedCount when status_changed event fires", async () => {
      // Set initial queuedCount
      useUiStore.setState({
        executionStatus: {
          ...useUiStore.getState().executionStatus,
          queuedCount: 5,
        },
      });

      renderHook(() => useExecutionEvents());

      await act(async () => {
        statusChangedHandler?.({
          payload: {
            isPaused: false,
            runningCount: 1,
            maxConcurrent: 2,
            reason: "task_started",
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.queuedCount).toBe(5);
    });
  });

  describe("queue_changed event", () => {
    it("registers listener for execution:queue_changed", () => {
      renderHook(() => useExecutionEvents());

      expect(listen).toHaveBeenCalledWith(
        "execution:queue_changed",
        expect.any(Function)
      );
    });

    it("updates only queuedCount on queue_changed event", async () => {
      // Set initial state
      useUiStore.setState({
        executionStatus: {
          isPaused: false,
          runningCount: 1,
          maxConcurrent: 2,
          queuedCount: 0,
          canStartTask: true,
        },
      });

      renderHook(() => useExecutionEvents());

      await act(async () => {
        queueChangedHandler?.({
          payload: {
            queuedCount: 3,
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      // queuedCount should be updated
      expect(state.queuedCount).toBe(3);
      // Other fields should be preserved
      expect(state.isPaused).toBe(false);
      expect(state.runningCount).toBe(1);
      expect(state.maxConcurrent).toBe(2);
      expect(state.canStartTask).toBe(true);
    });

    it("handles queuedCount decrement", async () => {
      useUiStore.setState({
        executionStatus: {
          ...useUiStore.getState().executionStatus,
          queuedCount: 5,
        },
      });

      renderHook(() => useExecutionEvents());

      await act(async () => {
        queueChangedHandler?.({
          payload: {
            queuedCount: 4,
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.queuedCount).toBe(4);
    });

    it("handles queuedCount set to zero", async () => {
      useUiStore.setState({
        executionStatus: {
          ...useUiStore.getState().executionStatus,
          queuedCount: 3,
        },
      });

      renderHook(() => useExecutionEvents());

      await act(async () => {
        queueChangedHandler?.({
          payload: {
            queuedCount: 0,
            timestamp: new Date().toISOString(),
          },
        });
      });

      const state = useUiStore.getState().executionStatus;
      expect(state.queuedCount).toBe(0);
    });
  });

  describe("cleanup on unmount", () => {
    it("cleans up status_changed listener on unmount", async () => {
      const { unmount } = renderHook(() => useExecutionEvents());

      // Allow promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      unmount();

      // Allow cleanup promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      expect(unlistenStatusMock).toHaveBeenCalled();
    });

    it("cleans up queue_changed listener on unmount", async () => {
      const { unmount } = renderHook(() => useExecutionEvents());

      // Allow promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      unmount();

      // Allow cleanup promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      expect(unlistenQueueMock).toHaveBeenCalled();
    });

    it("cleans up both listeners on unmount", async () => {
      const { unmount } = renderHook(() => useExecutionEvents());

      // Allow promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      unmount();

      // Allow cleanup promises to resolve
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });

      expect(unlistenStatusMock).toHaveBeenCalledTimes(1);
      expect(unlistenQueueMock).toHaveBeenCalledTimes(1);
    });
  });
});
