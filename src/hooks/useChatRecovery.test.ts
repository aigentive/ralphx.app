/**
 * useChatRecovery hook tests
 *
 * Tests recovery effects: agent running state sync, stuck-running cleanup,
 * and mount-time thrashing guard.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

// ============================================================================
// Mock infrastructure
// ============================================================================

const mockInvalidateQueries = vi.fn();

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mockInvalidateQueries,
  }),
}));

vi.mock("@/hooks/useChat", () => ({
  chatKeys: {
    conversation: (id: string) => ["chat", "conversations", id],
    conversationList: (type: string, id: string) => ["chat", "conversation-list", type, id],
  },
}));

vi.mock("@/hooks/useTasks", () => ({
  taskKeys: {
    list: (pid: string) => ["tasks", "list", pid],
    detail: (tid: string) => ["tasks", "detail", tid],
  },
}));

vi.mock("@/types/status", () => ({
  MERGE_STATUSES: ["pending_merge", "merging", "merge_conflict", "merge_incomplete"],
}));

vi.mock("@/api/chat", () => ({
  chatApi: {
    isAgentRunning: vi.fn(),
  },
}));

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useChatRecovery } from "./useChatRecovery";
import type { ContextType } from "@/types/chat-conversation";
import { chatApi } from "@/api/chat";

const mockIsAgentRunning = vi.mocked(chatApi.isAgentRunning);

// ============================================================================
// Helpers
// ============================================================================

interface DefaultProps {
  activeConversationId: string | null | undefined;
  storeContextKey: string;
  currentContextType: ContextType;
  currentContextId: string;
  isHistoryMode: boolean;
  isAgentContext: boolean;
  isAgentRunning: boolean;
  isConversationInCurrentContext: boolean;
  agentRunStatus: string | undefined;
  setAgentRunning: ReturnType<typeof vi.fn>;
  selectedTaskId: string | undefined;
  ideationSessionId: string | undefined;
  projectId: string;
  effectiveStatus: string | undefined;
}

function makeProps(overrides?: Partial<DefaultProps>): DefaultProps {
  return {
    activeConversationId: "conv-abc",
    storeContextKey: "task_execution:task-1",
    currentContextType: "task_execution" as ContextType,
    currentContextId: "task-1",
    isHistoryMode: false,
    isAgentContext: true,
    isAgentRunning: false,
    isConversationInCurrentContext: true,
    agentRunStatus: undefined,
    setAgentRunning: vi.fn(),
    selectedTaskId: "task-1",
    ideationSessionId: undefined,
    projectId: "project-1",
    effectiveStatus: "executing",
    ...overrides,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("useChatRecovery", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockInvalidateQueries.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("agent running state sync", () => {
    it("should set agent running when backend reports running status", () => {
      const props = makeProps({ agentRunStatus: "running" });
      renderHook(() => useChatRecovery(props));

      expect(props.setAgentRunning).toHaveBeenCalledWith("task_execution:task-1", true);
    });

    it("should NOT set agent running when status is not running", () => {
      const props = makeProps({ agentRunStatus: "completed" });
      renderHook(() => useChatRecovery(props));

      // Effect 1 shouldn't fire (status !== running), but effect 2 should fire with false
      const trueCalls = props.setAgentRunning.mock.calls.filter(
        (call: [string, boolean]) => call[1] === true
      );
      expect(trueCalls).toHaveLength(0);
    });
  });

  describe("stuck running state cleanup", () => {
    it("should clear running state when backend says completed", () => {
      const props = makeProps({ agentRunStatus: "completed" });
      renderHook(() => useChatRecovery(props));

      expect(props.setAgentRunning).toHaveBeenCalledWith("task_execution:task-1", false);
    });

    it("should NOT clear running state when agentRunStatus is undefined (loading)", () => {
      const props = makeProps({ agentRunStatus: undefined });
      renderHook(() => useChatRecovery(props));

      // Effect 2 should early-return when agentRunStatus === undefined
      const falseCalls = props.setAgentRunning.mock.calls.filter(
        (call: [string, boolean]) => call[1] === false
      );
      expect(falseCalls).toHaveLength(0);
    });

    it("should NOT clear when conversation is not in current context", () => {
      const props = makeProps({
        agentRunStatus: "completed",
        isConversationInCurrentContext: false,
      });
      renderHook(() => useChatRecovery(props));

      // Both effects should early-return
      expect(props.setAgentRunning).not.toHaveBeenCalled();
    });

    it("should NOT clear when no active conversation", () => {
      const props = makeProps({
        activeConversationId: null,
        agentRunStatus: "completed",
      });
      renderHook(() => useChatRecovery(props));

      // Effect 2 should early-return
      const falseCalls = props.setAgentRunning.mock.calls.filter(
        (call: [string, boolean]) => call[1] === false
      );
      expect(falseCalls).toHaveLength(0);
    });
  });

  describe("reconciliation poll (1.5s interval)", () => {
    beforeEach(() => {
      mockIsAgentRunning.mockClear();
    });

    it("should not start interval when isAgentRunning is false", () => {
      const props = makeProps({ isAgentRunning: false });
      renderHook(() => useChatRecovery(props));

      vi.advanceTimersByTime(3000);
      expect(mockIsAgentRunning).not.toHaveBeenCalled();
    });

    it("should poll is_agent_running every 1500ms when isAgentRunning is true", async () => {
      mockIsAgentRunning.mockResolvedValue(true);
      const props = makeProps({ isAgentRunning: true });
      renderHook(() => useChatRecovery(props));

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });
      expect(mockIsAgentRunning).toHaveBeenCalledTimes(1);
      expect(mockIsAgentRunning).toHaveBeenCalledWith("task_execution", "task-1");

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });
      expect(mockIsAgentRunning).toHaveBeenCalledTimes(2);
    });

    it("should clear stuck state when poll returns false", async () => {
      mockIsAgentRunning.mockResolvedValue(false);
      const props = makeProps({ isAgentRunning: true });
      renderHook(() => useChatRecovery(props));

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });

      expect(props.setAgentRunning).toHaveBeenCalledWith("task_execution:task-1", false);
    });

    it("should NOT clear state when poll returns true (agent still running)", async () => {
      mockIsAgentRunning.mockResolvedValue(true);
      const props = makeProps({ isAgentRunning: true });
      renderHook(() => useChatRecovery(props));

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });

      const falseCalls = props.setAgentRunning.mock.calls.filter(
        (call: [string, boolean]) => call[1] === false
      );
      expect(falseCalls).toHaveLength(0);
    });

    it("should clean up interval on unmount", async () => {
      mockIsAgentRunning.mockResolvedValue(true);
      const props = makeProps({ isAgentRunning: true });
      const { unmount } = renderHook(() => useChatRecovery(props));

      unmount();
      mockIsAgentRunning.mockClear();

      vi.advanceTimersByTime(3000);
      expect(mockIsAgentRunning).not.toHaveBeenCalled();
    });
  });

  describe("visibilitychange fast path", () => {
    beforeEach(() => {
      mockIsAgentRunning.mockClear();
    });

    it("should not attach listener when isAgentRunning is false", () => {
      const addEventSpy = vi.spyOn(document, "addEventListener");
      const props = makeProps({ isAgentRunning: false });
      renderHook(() => useChatRecovery(props));

      const visibilityCalls = addEventSpy.mock.calls.filter(
        ([event]) => event === "visibilitychange"
      );
      expect(visibilityCalls).toHaveLength(0);
      addEventSpy.mockRestore();
    });

    it("should reconcile immediately when app becomes visible and agent running", async () => {
      mockIsAgentRunning.mockResolvedValue(false);
      const props = makeProps({ isAgentRunning: true });
      renderHook(() => useChatRecovery(props));

      await act(async () => {
        Object.defineProperty(document, "visibilityState", {
          value: "visible",
          writable: true,
          configurable: true,
        });
        document.dispatchEvent(new Event("visibilitychange"));
      });

      expect(mockIsAgentRunning).toHaveBeenCalledWith("task_execution", "task-1");
      expect(props.setAgentRunning).toHaveBeenCalledWith("task_execution:task-1", false);
    });

    it("should remove listener on unmount", () => {
      mockIsAgentRunning.mockResolvedValue(true);
      const removeEventSpy = vi.spyOn(document, "removeEventListener");
      const props = makeProps({ isAgentRunning: true });
      const { unmount } = renderHook(() => useChatRecovery(props));

      unmount();

      const visibilityCalls = removeEventSpy.mock.calls.filter(
        ([event]) => event === "visibilitychange"
      );
      expect(visibilityCalls.length).toBeGreaterThan(0);
      removeEventSpy.mockRestore();
    });
  });
});
