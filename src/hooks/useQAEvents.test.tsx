/**
 * Tests for useQAEvents hook
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useQAEvents } from "./useQAEvents";
import { useQAStore } from "@/stores/qaStore";
import type { QAPrepEvent, QATestEvent } from "@/types/events";

// Mock Tauri event listener
const mockListeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: (event: { payload: unknown }) => void) => {
    mockListeners.set(eventName, callback);
    return Promise.resolve(() => {
      mockListeners.delete(eventName);
    });
  }),
}));

// Helper to emit events
function emitEvent(eventName: string, payload: unknown) {
  const listener = mockListeners.get(eventName);
  if (listener) {
    listener({ payload });
  }
}

describe("useQAEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListeners.clear();
    // Reset store state
    useQAStore.setState({
      settings: {
        qa_enabled: true,
        auto_qa_for_ui_tasks: true,
        auto_qa_for_api_tasks: false,
        qa_prep_enabled: true,
        browser_testing_enabled: true,
        browser_testing_url: "http://localhost:1420",
      },
      settingsLoaded: false,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  afterEach(() => {
    mockListeners.clear();
  });

  describe("listener registration", () => {
    it("should register qa:prep listener on mount", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });
    });

    it("should register qa:test listener on mount", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:test")).toBe(true);
      });
    });

    it("should unregister listeners on unmount", async () => {
      const { unmount } = renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      unmount();

      // Wait for cleanup
      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(false);
        expect(mockListeners.has("qa:test")).toBe(false);
      });
    });
  });

  describe("qa:prep events", () => {
    it("should handle qa_prep_started event", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      const prepEvent: QAPrepEvent = {
        taskId: "task-123",
        type: "started",
        agentId: "agent-456",
      };

      act(() => {
        emitEvent("qa:prep", prepEvent);
      });

      // Check store was updated - we expect setLoadingTask to have been called
      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(true);
    });

    it("should handle qa_prep_completed event", async () => {
      // First set up initial task QA data
      useQAStore.getState().setTaskQA("task-123", {
        id: "qa-1",
        task_id: "task-123",
        screenshots: [],
        created_at: new Date().toISOString(),
      });

      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      const prepEvent: QAPrepEvent = {
        taskId: "task-123",
        type: "completed",
        agentId: "agent-456",
        acceptanceCriteriaCount: 5,
        testStepsCount: 10,
      };

      act(() => {
        emitEvent("qa:prep", prepEvent);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(false);
    });

    it("should handle qa_prep_failed event", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      const prepEvent: QAPrepEvent = {
        taskId: "task-123",
        type: "failed",
        error: "Failed to generate acceptance criteria",
      };

      act(() => {
        emitEvent("qa:prep", prepEvent);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(false);
      expect(state.error).toBe("QA Prep failed for task task-123: Failed to generate acceptance criteria");
    });

    it("should ignore invalid qa:prep events", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      const invalidEvent = { invalid: "data" };

      act(() => {
        emitEvent("qa:prep", invalidEvent);
      });

      expect(consoleSpy).toHaveBeenCalledWith(
        "Invalid QA prep event:",
        expect.any(String)
      );

      consoleSpy.mockRestore();
    });
  });

  describe("qa:test events", () => {
    it("should handle qa_testing_started event", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:test")).toBe(true);
      });

      const testEvent: QATestEvent = {
        taskId: "task-123",
        type: "started",
        agentId: "agent-789",
      };

      act(() => {
        emitEvent("qa:test", testEvent);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(true);
    });

    it("should handle qa_passed event", async () => {
      useQAStore.getState().setTaskQA("task-123", {
        id: "qa-1",
        task_id: "task-123",
        screenshots: [],
        created_at: new Date().toISOString(),
      });

      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:test")).toBe(true);
      });

      const testEvent: QATestEvent = {
        taskId: "task-123",
        type: "passed",
        totalSteps: 5,
        passedSteps: 5,
        failedSteps: 0,
      };

      act(() => {
        emitEvent("qa:test", testEvent);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(false);
    });

    it("should handle qa_failed event", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:test")).toBe(true);
      });

      const testEvent: QATestEvent = {
        taskId: "task-123",
        type: "failed",
        totalSteps: 5,
        passedSteps: 3,
        failedSteps: 2,
        error: "2 tests failed",
      };

      act(() => {
        emitEvent("qa:test", testEvent);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(false);
      expect(state.error).toBe("QA Tests failed for task task-123: 2 tests failed");
    });

    it("should ignore invalid qa:test events", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:test")).toBe(true);
      });

      const invalidEvent = { invalid: "data" };

      act(() => {
        emitEvent("qa:test", invalidEvent);
      });

      expect(consoleSpy).toHaveBeenCalledWith(
        "Invalid QA test event:",
        expect.any(String)
      );

      consoleSpy.mockRestore();
    });
  });

  describe("taskId filtering", () => {
    it("should filter events by taskId when provided", async () => {
      renderHook(() => useQAEvents("task-123"));

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      // This event should be handled
      const matchingEvent: QAPrepEvent = {
        taskId: "task-123",
        type: "started",
      };

      // This event should be ignored
      const nonMatchingEvent: QAPrepEvent = {
        taskId: "task-456",
        type: "started",
      };

      act(() => {
        emitEvent("qa:prep", matchingEvent);
      });

      let state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(true);

      act(() => {
        emitEvent("qa:prep", nonMatchingEvent);
      });

      state = useQAStore.getState();
      // task-456 should NOT be marked as loading
      expect(state.loadingTasks.has("task-456")).toBe(false);
    });

    it("should handle all events when taskId is not provided", async () => {
      renderHook(() => useQAEvents());

      await waitFor(() => {
        expect(mockListeners.has("qa:prep")).toBe(true);
      });

      const event1: QAPrepEvent = { taskId: "task-123", type: "started" };
      const event2: QAPrepEvent = { taskId: "task-456", type: "started" };

      act(() => {
        emitEvent("qa:prep", event1);
        emitEvent("qa:prep", event2);
      });

      const state = useQAStore.getState();
      expect(state.loadingTasks.has("task-123")).toBe(true);
      expect(state.loadingTasks.has("task-456")).toBe(true);
    });
  });
});
