import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  useTaskEvents,
  useAgentEvents,
  useSupervisorAlerts,
  useReviewEvents,
} from "./useEvents";
import { useTaskStore } from "@/stores/taskStore";
import { useActivityStore } from "@/stores/activityStore";
import type { Task } from "@/types/task";
import type { AgentMessageEvent } from "@/types/events";

// Mock Tauri event API
const mockUnlisten = vi.fn();
const mockListen = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

// Valid UUID for testing
const TASK_UUID = "123e4567-e89b-12d3-a456-426614174000";
const PROJECT_UUID = "123e4567-e89b-12d3-a456-426614174001";
const NEW_TASK_UUID = "123e4567-e89b-12d3-a456-426614174002";

// Helper to create a mock task
const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: TASK_UUID,
  projectId: PROJECT_UUID,
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

describe("useTaskEvents", () => {
  const eventCallbacks: Record<string, (event: { payload: unknown }) => void> = {};

  beforeEach(() => {
    Object.keys(eventCallbacks).forEach(key => delete eventCallbacks[key]);
    mockListen.mockReset();
    mockUnlisten.mockReset();

    // Setup mock to capture the callback and return an unlisten function
    mockListen.mockImplementation(
      (eventName: string, callback: (event: { payload: unknown }) => void) => {
        eventCallbacks[eventName] = callback;
        return Promise.resolve(mockUnlisten as unknown as UnlistenFn);
      }
    );

    // Reset the task store
    useTaskStore.setState({ tasks: {}, selectedTaskId: null });
  });

  afterEach(() => {
    Object.keys(eventCallbacks).forEach(key => delete eventCallbacks[key]);
  });

  it("should set up event listener on mount", () => {
    renderHook(() => useTaskEvents());

    expect(mockListen).toHaveBeenCalledTimes(2);
    expect(mockListen).toHaveBeenCalledWith("task:event", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("task:status_changed", expect.any(Function));
  });

  it("should clean up listener on unmount", async () => {
    const { unmount } = renderHook(() => useTaskEvents());

    unmount();

    // Wait for the cleanup to be called
    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalledTimes(2);
    });
  });

  it("should handle created event by adding task to store", async () => {
    renderHook(() => useTaskEvents());

    const newTask = createMockTask({ id: NEW_TASK_UUID });

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "created",
          task: newTask,
        },
      });
    });

    const state = useTaskStore.getState();
    expect(state.tasks[NEW_TASK_UUID]).toEqual(newTask);
  });

  it("should handle updated event by updating task in store", async () => {
    // Pre-populate store with a task
    useTaskStore.setState({
      tasks: {
        [TASK_UUID]: createMockTask({ id: TASK_UUID, title: "Original" }),
      },
    });

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "updated",
          taskId: TASK_UUID,
          changes: { title: "Updated Title" },
        },
      });
    });

    const state = useTaskStore.getState();
    expect(state.tasks[TASK_UUID]?.title).toBe("Updated Title");
  });

  it("should handle deleted event by removing task from store", async () => {
    // Pre-populate store with a task
    useTaskStore.setState({
      tasks: {
        [TASK_UUID]: createMockTask({ id: TASK_UUID }),
      },
    });

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "deleted",
          taskId: TASK_UUID,
        },
      });
    });

    const state = useTaskStore.getState();
    expect(state.tasks[TASK_UUID]).toBeUndefined();
  });

  it("should handle status_changed event by updating task status", async () => {
    // Pre-populate store with a task
    useTaskStore.setState({
      tasks: {
        [TASK_UUID]: createMockTask({ id: TASK_UUID, internalStatus: "backlog" }),
      },
    });

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "status_changed",
          taskId: TASK_UUID,
          from: "backlog",
          to: "ready",
          changedBy: "user",
        },
      });
    });

    const state = useTaskStore.getState();
    expect(state.tasks[TASK_UUID]?.internalStatus).toBe("ready");
  });

  it("should handle task:status_changed event by updating task status", async () => {
    useTaskStore.setState({
      tasks: {
        [TASK_UUID]: createMockTask({ id: TASK_UUID, internalStatus: "pending_merge" }),
      },
    });

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:status_changed"]?.({
        payload: {
          task_id: TASK_UUID,
          old_status: "pending_merge",
          new_status: "merged",
        },
      });
    });

    const state = useTaskStore.getState();
    expect(state.tasks[TASK_UUID]?.internalStatus).toBe("merged");
  });

  it("should log error for invalid event payload", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "invalid_type",
          someField: "value",
        },
      });
    });

    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it("should log error for malformed event payload", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    renderHook(() => useTaskEvents());

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: "not an object",
      });
    });

    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it("should not update store for non-existent task on updated event", async () => {
    // Start with empty store
    useTaskStore.setState({ tasks: {} });
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    renderHook(() => useTaskEvents());

    const nonExistentUuid = "123e4567-e89b-12d3-a456-426614174099";

    await act(async () => {
      eventCallbacks["task:event"]?.({
        payload: {
          type: "updated",
          taskId: nonExistentUuid,
          changes: { title: "Updated" },
        },
      });
    });

    const state = useTaskStore.getState();
    expect(Object.keys(state.tasks)).toHaveLength(0);
    consoleSpy.mockRestore();
  });
});

describe("useAgentEvents", () => {
  // Store event callbacks per event name
  const eventCallbacks: Record<string, (event: { payload: unknown }) => void> = {};

  beforeEach(() => {
    Object.keys(eventCallbacks).forEach(key => delete eventCallbacks[key]);
    mockListen.mockReset();
    mockUnlisten.mockReset();

    mockListen.mockImplementation(
      (eventName: string, callback: (event: { payload: unknown }) => void) => {
        eventCallbacks[eventName] = callback;
        return Promise.resolve(mockUnlisten as unknown as UnlistenFn);
      }
    );

    // Reset activity store
    useActivityStore.setState({ messages: [], alerts: [] });
  });

  it("should set up listener for agent:message event", () => {
    renderHook(() => useAgentEvents());

    expect(mockListen).toHaveBeenCalledWith("agent:message", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("agent:message_created", expect.any(Function));
  });

  it("should add message to activity store", async () => {
    renderHook(() => useAgentEvents());

    const message: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "Processing...",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallbacks["agent:message"]?.({ payload: message });
    });

    const state = useActivityStore.getState();
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]?.content).toBe("Processing...");
  });

  it("should add message from agent:message_created for task execution", async () => {
    renderHook(() => useAgentEvents());

    await act(async () => {
      eventCallbacks["agent:message_created"]?.({
        payload: {
          context_type: "task_execution",
          context_id: TASK_UUID,
          content: "Final output",
        },
      });
    });

    const state = useActivityStore.getState();
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]?.content).toBe("Final output");
  });

  it("should filter messages by taskId when provided", async () => {
    renderHook(() => useAgentEvents(TASK_UUID));

    const matchingMessage: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "Matching",
      timestamp: Date.now(),
    };

    const nonMatchingMessage: AgentMessageEvent = {
      taskId: NEW_TASK_UUID,
      type: "thinking",
      content: "Non-matching",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallbacks["agent:message"]?.({ payload: matchingMessage });
      eventCallbacks["agent:message"]?.({ payload: nonMatchingMessage });
    });

    const state = useActivityStore.getState();
    expect(state.messages).toHaveLength(1);
    expect(state.messages[0]?.content).toBe("Matching");
  });

  it("should receive all messages when no taskId filter provided", async () => {
    renderHook(() => useAgentEvents());

    const message1: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "First",
      timestamp: Date.now(),
    };

    const message2: AgentMessageEvent = {
      taskId: NEW_TASK_UUID,
      type: "text",
      content: "Second",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallbacks["agent:message"]?.({ payload: message1 });
      eventCallbacks["agent:message"]?.({ payload: message2 });
    });

    const state = useActivityStore.getState();
    expect(state.messages).toHaveLength(2);
  });

  it("should clean up listener on unmount", async () => {
    const { unmount } = renderHook(() => useAgentEvents());

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });
});

describe("useSupervisorAlerts", () => {
  const eventCallbacks: Record<string, (event: { payload: unknown }) => void> = {};

  beforeEach(() => {
    Object.keys(eventCallbacks).forEach(key => delete eventCallbacks[key]);
    mockListen.mockReset();
    mockUnlisten.mockReset();

    mockListen.mockImplementation(
      (eventName: string, callback: (event: { payload: unknown }) => void) => {
        eventCallbacks[eventName] = callback;
        return Promise.resolve(mockUnlisten as unknown as UnlistenFn);
      }
    );

    // Reset activity store
    useActivityStore.setState({ messages: [], alerts: [] });
  });

  it("should set up listener for supervisor:alert event", () => {
    renderHook(() => useSupervisorAlerts());

    expect(mockListen).toHaveBeenCalledWith("supervisor:alert", expect.any(Function));
  });

  it("should add alert to activity store", async () => {
    renderHook(() => useSupervisorAlerts());

    const alert = {
      taskId: TASK_UUID,
      severity: "high",
      type: "error",
      message: "Something went wrong",
    };

    await act(async () => {
      eventCallbacks["supervisor:alert"]?.({ payload: alert });
    });

    const state = useActivityStore.getState();
    expect(state.alerts).toHaveLength(1);
    expect(state.alerts[0]?.message).toBe("Something went wrong");
  });

  it("should report unread alerts for high/critical severity", async () => {
    renderHook(() => useSupervisorAlerts());

    const criticalAlert = {
      taskId: TASK_UUID,
      severity: "critical",
      type: "stuck",
      message: "Critical issue",
    };

    await act(async () => {
      eventCallbacks["supervisor:alert"]?.({ payload: criticalAlert });
    });

    const state = useActivityStore.getState();
    expect(state.hasUnreadAlerts()).toBe(true);
  });

  it("should clean up listener on unmount", async () => {
    const { unmount } = renderHook(() => useSupervisorAlerts());

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });
});

describe("useReviewEvents", () => {
  const eventCallbacks: Record<string, (event: { payload: unknown }) => void> = {};
  const mockInvalidateQueries = vi.fn();

  beforeEach(() => {
    Object.keys(eventCallbacks).forEach((key) => delete eventCallbacks[key]);
    mockListen.mockReset();
    mockUnlisten.mockReset();
    mockInvalidateQueries.mockReset();

    mockListen.mockImplementation(
      (eventName: string, callback: (event: { payload: unknown }) => void) => {
        eventCallbacks[eventName] = callback;
        return Promise.resolve(mockUnlisten as unknown as UnlistenFn);
      }
    );
  });

  // Helper to render hook with QueryClient wrapper
  const createWrapper = () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    // Mock invalidateQueries
    queryClient.invalidateQueries = mockInvalidateQueries;

    return ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };

  it("should set up listener for review:update event", () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    expect(mockListen).toHaveBeenCalledWith("review:update", expect.any(Function));
  });

  it("should clean up listener on unmount", async () => {
    const { unmount } = renderHook(() => useReviewEvents(), {
      wrapper: createWrapper(),
    });

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });

  it("should invalidate pending reviews query on review:started event", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const reviewEvent = {
      taskId: TASK_UUID,
      reviewId: "review-123",
      type: "started",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "pending"],
    });
  });

  it("should invalidate task reviews query on review:completed event", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const reviewEvent = {
      taskId: TASK_UUID,
      reviewId: "review-123",
      type: "completed",
      outcome: "approved",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "byTask", TASK_UUID],
    });
    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "pending"],
    });
  });

  it("should invalidate state history query on review:completed event", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const reviewEvent = {
      taskId: TASK_UUID,
      reviewId: "review-123",
      type: "completed",
      outcome: "approved",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "stateHistory", TASK_UUID],
    });
  });

  it("should invalidate pending reviews on needs_human event", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const reviewEvent = {
      taskId: TASK_UUID,
      reviewId: "review-123",
      type: "needs_human",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "pending"],
    });
  });

  it("should invalidate pending reviews on fix_proposed event", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const reviewEvent = {
      taskId: TASK_UUID,
      reviewId: "review-123",
      type: "fix_proposed",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "pending"],
    });
  });

  it("should log error for invalid event payload", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    await act(async () => {
      eventCallbacks["review:update"]?.({
        payload: { invalid: "payload" },
      });
    });

    expect(consoleSpy).toHaveBeenCalled();
    expect(mockInvalidateQueries).not.toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it("should handle all review event types", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const eventTypes = ["started", "completed", "needs_human", "fix_proposed"] as const;

    for (const type of eventTypes) {
      mockInvalidateQueries.mockClear();

      const reviewEvent = {
        taskId: TASK_UUID,
        reviewId: "review-123",
        type,
        ...(type === "completed" && { outcome: "approved" }),
      };

      await act(async () => {
        eventCallbacks["review:update"]?.({ payload: reviewEvent });
      });

      // All events should at least invalidate pending reviews
      expect(mockInvalidateQueries).toHaveBeenCalledWith({
        queryKey: ["reviews", "pending"],
      });
    }
  });

  it("should invalidate task-specific queries with correct taskId", async () => {
    renderHook(() => useReviewEvents(), { wrapper: createWrapper() });

    const specificTaskId = "specific-task-uuid";
    const reviewEvent = {
      taskId: specificTaskId,
      reviewId: "review-123",
      type: "completed",
      outcome: "changes_requested",
    };

    await act(async () => {
      eventCallbacks["review:update"]?.({ payload: reviewEvent });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "byTask", specificTaskId],
    });
    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["reviews", "stateHistory", specificTaskId],
    });
  });
});
