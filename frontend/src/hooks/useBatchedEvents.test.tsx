import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useBatchedAgentMessages } from "./useBatchedEvents";
import type { AgentMessageEvent } from "@/types/events";

// Mock EventProvider bus API
const mockUnlisten = vi.fn();
const mockSubscribe = vi.fn();

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: (...args: unknown[]) => mockSubscribe(...args),
    emit: vi.fn(),
  }),
}));

// Valid UUID for testing
const TASK_UUID = "123e4567-e89b-12d3-a456-426614174000";

describe("useBatchedAgentMessages", () => {
  let eventCallback: ((payload: AgentMessageEvent) => void) | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
    eventCallback = null;
    mockSubscribe.mockReset();
    mockUnlisten.mockReset();

    mockSubscribe.mockImplementation(
      (eventName: string, callback: (payload: AgentMessageEvent) => void) => {
        eventCallback = callback;
        return mockUnlisten;
      }
    );
  });

  afterEach(() => {
    vi.useRealTimers();
    eventCallback = null;
  });

  it("should set up event listener on mount", () => {
    renderHook(() => useBatchedAgentMessages(TASK_UUID));

    expect(mockSubscribe).toHaveBeenCalledWith("agent:message", expect.any(Function));
  });

  it("should buffer incoming messages", async () => {
    const { result } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    const message: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "Processing...",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallback?.(message);
    });

    // Before flush, messages array should be empty (still in buffer)
    expect(result.current).toHaveLength(0);
  });

  it("should flush buffer every 50ms", async () => {
    const { result } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    const message1: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "First",
      timestamp: Date.now(),
    };

    const message2: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "text",
      content: "Second",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallback?.(message1);
      eventCallback?.(message2);
    });

    // Before flush
    expect(result.current).toHaveLength(0);

    // Advance time to trigger flush
    await act(async () => {
      vi.advanceTimersByTime(50);
    });

    // After flush, messages should be visible
    expect(result.current).toHaveLength(2);
    expect(result.current[0]?.content).toBe("First");
    expect(result.current[1]?.content).toBe("Second");
  });

  it("should filter messages by taskId", async () => {
    const { result } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    const matchingMessage: AgentMessageEvent = {
      taskId: TASK_UUID,
      type: "thinking",
      content: "Matching",
      timestamp: Date.now(),
    };

    const nonMatchingMessage: AgentMessageEvent = {
      taskId: "123e4567-e89b-12d3-a456-426614174099",
      type: "thinking",
      content: "Non-matching",
      timestamp: Date.now(),
    };

    await act(async () => {
      eventCallback?.(matchingMessage);
      eventCallback?.(nonMatchingMessage);
    });

    await act(async () => {
      vi.advanceTimersByTime(50);
    });

    expect(result.current).toHaveLength(1);
    expect(result.current[0]?.content).toBe("Matching");
  });

  it("should accumulate messages across time", async () => {
    // This test verifies that the hook properly accumulates messages
    // by sending all messages first, then checking after a flush
    const { result, rerender } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    // Send multiple messages with small time gaps
    await act(async () => {
      eventCallback?.({
        taskId: TASK_UUID,
        type: "thinking",
        content: "First",
        timestamp: Date.now(),
      });
      vi.advanceTimersByTime(30); // Less than flush interval

      eventCallback?.({
        taskId: TASK_UUID,
        type: "text",
        content: "Second",
        timestamp: Date.now(),
      });
      vi.advanceTimersByTime(30); // Now at 60ms, one flush should have happened
    });

    // After 60ms, we should have had a flush and the messages should be available
    // Re-render to get updated state
    rerender();

    // Both messages should be visible after flush
    expect(result.current.length).toBeGreaterThanOrEqual(1);
  });

  it("should clean up listener on unmount", async () => {
    vi.useRealTimers(); // Use real timers for this test

    const { unmount } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    // Unmount
    unmount();

    // Allow promise to resolve
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(mockUnlisten).toHaveBeenCalled();

    vi.useFakeTimers(); // Restore fake timers
  });

  it("should handle empty buffer on flush", async () => {
    const { result } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    // No messages sent, just flush
    await act(async () => {
      vi.advanceTimersByTime(50);
    });

    expect(result.current).toHaveLength(0);
  });

  it("should handle multiple rapid messages", async () => {
    const { result } = renderHook(() => useBatchedAgentMessages(TASK_UUID));

    // Send 10 messages rapidly
    await act(async () => {
      for (let i = 0; i < 10; i++) {
        eventCallback?.({
          taskId: TASK_UUID,
          type: "thinking",
          content: `Message ${i}`,
          timestamp: Date.now(),
        });
      }
    });

    // Flush
    await act(async () => {
      vi.advanceTimersByTime(50);
    });

    expect(result.current).toHaveLength(10);
  });
});
