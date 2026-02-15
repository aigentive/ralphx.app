/**
 * usePaneEvents tests — Auto-focus on agent activity events
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { usePaneEvents } from "./usePaneEvents";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import type { EventHandler, Unsubscribe } from "@/lib/event-bus";

// ============================================================================
// Mocks
// ============================================================================

type SubscriptionEntry = { event: string; handler: EventHandler };
let subscriptions: SubscriptionEntry[] = [];

const mockBus = {
  subscribe: vi.fn(<T,>(event: string, handler: EventHandler<T>): Unsubscribe => {
    const entry = { event, handler: handler as EventHandler };
    subscriptions.push(entry);
    return () => {
      subscriptions = subscriptions.filter((s) => s !== entry);
    };
  }),
  emit: vi.fn(),
};

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => mockBus,
}));

vi.mock("@/lib/chat-context-registry", () => ({
  buildStoreKey: (contextType: string, contextId: string) => `${contextType}:${contextId}`,
}));

// ============================================================================
// Helpers
// ============================================================================

function resetStore() {
  useSplitPaneStore.setState({
    isActive: true,
    focusedPane: null,
    coordinatorWidth: 40,
    isPrefixKeyActive: false,
    contextKey: null,
    paneOrder: ["coder-1", "coder-2"],
    panes: {
      "coder-1": { streaming: false, minimized: false, maximized: false, unreadCount: 0 },
      "coder-2": { streaming: false, minimized: false, maximized: false, unreadCount: 0 },
    },
  });
}

function emitEvent(eventName: string, payload: Record<string, unknown>) {
  for (const sub of subscriptions) {
    if (sub.event === eventName) {
      sub.handler(payload);
    }
  }
}

// ============================================================================
// Tests
// ============================================================================

describe("usePaneEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    subscriptions = [];
    resetStore();
  });

  it("does not subscribe when contextKey is null", () => {
    renderHook(() => usePaneEvents(null, "coder-1", { autoFocus: true }));
    expect(subscriptions).toHaveLength(0);
  });

  it("does not subscribe when teammateName is null", () => {
    renderHook(() => usePaneEvents("task_execution:task-1", null, { autoFocus: true }));
    expect(subscriptions).toHaveLength(0);
  });

  it("subscribes to agent:run_started and agent:chunk events", () => {
    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    const events = subscriptions.map((s) => s.event);
    expect(events).toContain("agent:run_started");
    expect(events).toContain("agent:chunk");
  });

  it("auto-focuses pane on run_started when autoFocus enabled", () => {
    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    emitEvent("agent:run_started", {
      context_type: "task_execution",
      context_id: "task-1",
      teammate_name: "coder-1",
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("coder-1");
  });

  it("auto-focuses on agent:chunk only when no pane is focused", () => {
    // No pane focused (null)
    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    emitEvent("agent:chunk", {
      context_type: "task_execution",
      context_id: "task-1",
      teammate_name: "coder-1",
      text: "hello",
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("coder-1");
  });

  it("does NOT auto-focus on agent:chunk when another pane is focused", () => {
    useSplitPaneStore.setState({ focusedPane: "coder-2" });

    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    emitEvent("agent:chunk", {
      context_type: "task_execution",
      context_id: "task-1",
      teammate_name: "coder-1",
      text: "hello",
    });

    // Should stay on coder-2, not switch to coder-1
    expect(useSplitPaneStore.getState().focusedPane).toBe("coder-2");
  });

  it("does not auto-focus when autoFocus is disabled", () => {
    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: false }),
    );

    emitEvent("agent:run_started", {
      context_type: "task_execution",
      context_id: "task-1",
      teammate_name: "coder-1",
    });

    expect(useSplitPaneStore.getState().focusedPane).toBeNull();
  });

  it("ignores events from different context", () => {
    renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    emitEvent("agent:run_started", {
      context_type: "task_execution",
      context_id: "task-OTHER",
      teammate_name: "coder-1",
    });

    expect(useSplitPaneStore.getState().focusedPane).toBeNull();
  });

  it("unsubscribes on unmount", () => {
    const { unmount } = renderHook(() =>
      usePaneEvents("task_execution:task-1", "coder-1", { autoFocus: true }),
    );

    expect(subscriptions.length).toBeGreaterThan(0);

    unmount();

    expect(subscriptions).toHaveLength(0);
  });
});
