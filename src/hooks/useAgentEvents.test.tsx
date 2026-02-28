/**
 * Tests for useAgentEvents hook
 *
 * Covers:
 * - agent:run_started sets running state
 * - agent:run_completed clears running state
 * - agent:stopped clears running state (defensive)
 * - agent:error clears running state
 * - Event listeners are properly cleaned up on unmount
 */

import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useAgentEvents } from "./useAgentEvents";
import { useChatStore } from "@/stores/chatStore";

// ============================================================================
// Mock EventBus
// ============================================================================

type EventHandler = (payload: unknown) => void;

const listeners = new Map<string, Set<EventHandler>>();

function mockSubscribe(event: string, handler: EventHandler) {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event)!.add(handler);
  return () => {
    listeners.get(event)?.delete(handler);
  };
}

function emitEvent(event: string, payload: unknown) {
  listeners.get(event)?.forEach((handler) => handler(payload));
}

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
    emit: vi.fn(),
  }),
}));

// Mock useChat to provide chatKeys
vi.mock("@/hooks/useChat", () => ({
  chatKeys: {
    conversationList: (type: string, id: string) => ["chat", "conversations", type, id],
    conversation: (id: string) => ["chat", "conversation", id],
    agentRun: (id: string) => ["chat", "agentRun", id],
  },
}));

// ============================================================================
// Test Setup
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function createWrapperWithClient() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { queryClient, wrapper };
}

describe("useAgentEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();

    // Reset chat store
    useChatStore.setState({
      messages: {},
      context: null,
      width: 320,
      isLoading: false,
      activeConversationId: null,
      queuedMessages: {},
      agentStatus: {},
      isSending: {},
    });
  });

  describe("agent:run_started", () => {
    it("sets agent running state for the event context", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      const state = useChatStore.getState();
      expect(state.agentStatus["task:task-123"]).toBe("generating");
    });

    it("sets running state for task_execution context", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      const state = useChatStore.getState();
      expect(state.agentStatus["task_execution:task-123"]).toBe("generating");
    });

    it("sets running state for review context", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      const state = useChatStore.getState();
      expect(state.agentStatus["review:task-123"]).toBe("generating");
    });

    it("sets running state for merge context", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "merge",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      const state = useChatStore.getState();
      expect(state.agentStatus["merge:task-123"]).toBe("generating");
    });

    it("sets running state for ideation context", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "ideation",
          context_id: "session-789",
          conversation_id: "conv-1",
        });
      });

      const state = useChatStore.getState();
      expect(state.agentStatus["session:session-789"]).toBe("generating");
    });
  });

  describe("agent:run_completed", () => {
    it("clears agent running state on completion", () => {
      const wrapper = createWrapper();

      // First set running state
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-123", true);
      });
      expect(useChatStore.getState().agentStatus["task:task-123"]).toBe("generating");

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "task",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      const state = useChatStore.getState();
      // After run_completed, the running state should be cleared
      expect(state.agentStatus["task:task-123"]).toBeUndefined();
    });

    it("clears running state for task_execution on stop/completion", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("task_execution:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBeUndefined();
    });

    it("clears running state for review on stop/completion", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("review:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      expect(useChatStore.getState().agentStatus["review:task-123"]).toBeUndefined();
    });

    it("clears running state for ideation on stop/completion", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("session:session-789", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "ideation",
          context_id: "session-789",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      expect(useChatStore.getState().agentStatus["session:session-789"]).toBeUndefined();
    });

    it("clears running state for merge on stop/completion", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("merge:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "merge",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      expect(useChatStore.getState().agentStatus["merge:task-123"]).toBeUndefined();
    });
  });

  describe("agent:stopped", () => {
    it("clears agent running state on stop (defensive)", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("task:task-123", true);
      });
      expect(useChatStore.getState().agentStatus["task:task-123"]).toBe("generating");

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:stopped", {
          context_type: "task",
          context_id: "task-123",
          conversation_id: "conv-1",
          agent_run_id: "run-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task:task-123"]).toBeUndefined();
    });

    it("clears running state for task_execution on stop", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("task_execution:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:stopped", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          agent_run_id: "run-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBeUndefined();
    });

    it("clears running state for review on stop", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("review:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:stopped", {
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
          agent_run_id: "run-1",
        });
      });

      expect(useChatStore.getState().agentStatus["review:task-123"]).toBeUndefined();
    });
  });

  describe("agent:error", () => {
    it("clears agent running state on error", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("task:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:error", {
          context_type: "task",
          context_id: "task-123",
          conversation_id: "conv-1",
          error: "Something went wrong",
        });
      });

      expect(useChatStore.getState().agentStatus["task:task-123"]).toBeUndefined();
    });

    it("clears running state for task_execution on error", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("task_execution:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:error", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          error: "Agent crashed",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBeUndefined();
    });

    it("clears running state for ideation on error", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("session:session-789", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:error", {
          context_type: "ideation",
          context_id: "session-789",
          conversation_id: "conv-1",
          error: "Session error",
        });
      });

      expect(useChatStore.getState().agentStatus["session:session-789"]).toBeUndefined();
    });

    it("clears running state for review on error", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("review:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:error", {
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
          error: "Review failed",
        });
      });

      expect(useChatStore.getState().agentStatus["review:task-123"]).toBeUndefined();
    });

    it("clears running state for merge on error", () => {
      const wrapper = createWrapper();

      act(() => {
        useChatStore.getState().setAgentRunning("merge:task-123", true);
      });

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:error", {
          context_type: "merge",
          context_id: "task-123",
          conversation_id: "conv-1",
          error: "Merge conflict",
        });
      });

      expect(useChatStore.getState().agentStatus["merge:task-123"]).toBeUndefined();
    });
  });

  describe("agent:turn_completed", () => {
    it("sets waiting_for_input for task_execution — agent stays alive between turns", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      // Transitions to waiting_for_input — agent alive, not generating
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("waiting_for_input");
    });

    it("sets waiting_for_input for ideation — agent stays alive between turns", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "ideation",
          context_id: "session-789",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["session:session-789"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "ideation",
          context_id: "session-789",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      expect(useChatStore.getState().agentStatus["session:session-789"]).toBe("waiting_for_input");
    });

    it("sets waiting_for_input for review — agent stays alive between turns", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["review:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "review",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      expect(useChatStore.getState().agentStatus["review:task-123"]).toBe("waiting_for_input");
    });

    it("sets waiting_for_input for merge — agent stays alive between turns", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "merge",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["merge:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "merge",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      expect(useChatStore.getState().agentStatus["merge:task-123"]).toBe("waiting_for_input");
    });

    it("invalidates agentRun and conversation queries when conversation_id matches active", () => {
      const { queryClient, wrapper } = createWrapperWithClient();
      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: ["chat", "agentRun", "conv-1"],
      });
      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: ["chat", "conversation", "conv-1"],
      });
    });

    it("does not invalidate queries when conversation_id does not match active", () => {
      const { queryClient, wrapper } = createWrapperWithClient();
      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-OTHER",
          status: "turn_complete",
        });
      });

      expect(invalidateSpy).not.toHaveBeenCalled();
    });

    it("skips event and does not invalidate queries when teammate_name is set", () => {
      const { queryClient, wrapper } = createWrapperWithClient();
      const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
          teammate_name: "researcher",
        });
      });

      // No queries invalidated — teammate event was skipped
      expect(invalidateSpy).not.toHaveBeenCalled();
    });
  });

  describe("turn_completed → run_completed sequence (process dies between turns)", () => {
    it("run_started → turn_completed → run_completed settles isAgentRunning=false", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      // Turn completed — waiting for user input
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("waiting_for_input");

      act(() => {
        emitEvent("agent:run_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "completed",
        });
      });

      // Process died — should be cleared
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBeUndefined();
    });

    it("rapid burst: turn_completed ×3 keeps agent alive throughout", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      // Still alive after burst — waiting for user input (last turn_completed wins)
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("waiting_for_input");
    });

    it("turn_completed followed by agent:error clears isAgentRunning", () => {
      const wrapper = createWrapper();
      renderHook(() => useAgentEvents("conv-1"), { wrapper });

      act(() => {
        emitEvent("agent:run_started", {
          run_id: "run-1",
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
        });
      });

      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("generating");

      act(() => {
        emitEvent("agent:turn_completed", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          status: "turn_complete",
        });
      });

      // Turn completed — waiting for user input
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBe("waiting_for_input");

      act(() => {
        emitEvent("agent:error", {
          context_type: "task_execution",
          context_id: "task-123",
          conversation_id: "conv-1",
          error: "Process crashed after turn",
        });
      });

      // Error clears the running state
      expect(useChatStore.getState().agentStatus["task_execution:task-123"]).toBeUndefined();
    });
  });

  describe("cleanup", () => {
    it("unsubscribes from events on unmount", () => {
      const wrapper = createWrapper();
      const { unmount } = renderHook(() => useAgentEvents("conv-1"), { wrapper });

      // Events should be registered
      expect(listeners.get("agent:run_started")?.size).toBe(1);
      expect(listeners.get("agent:run_completed")?.size).toBe(1);
      expect(listeners.get("agent:stopped")?.size).toBe(1);
      expect(listeners.get("agent:error")?.size).toBe(1);

      unmount();

      // After unmount, listeners should be cleared
      expect(listeners.get("agent:run_started")?.size ?? 0).toBe(0);
      expect(listeners.get("agent:run_completed")?.size ?? 0).toBe(0);
      expect(listeners.get("agent:stopped")?.size ?? 0).toBe(0);
      expect(listeners.get("agent:error")?.size ?? 0).toBe(0);
    });

    it("registers turn_completed listener on mount and unregisters on unmount", () => {
      const wrapper = createWrapper();
      const { unmount } = renderHook(() => useAgentEvents("conv-1"), { wrapper });

      expect(listeners.get("agent:turn_completed")?.size).toBe(1);

      unmount();

      expect(listeners.get("agent:turn_completed")?.size ?? 0).toBe(0);
    });
  });
});
