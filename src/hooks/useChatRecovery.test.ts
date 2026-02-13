/**
 * useChatRecovery hook tests
 *
 * Tests recovery effects: agent running state sync, stuck-running cleanup,
 * and mount-time thrashing guard.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";

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

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useChatRecovery } from "./useChatRecovery";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Helpers
// ============================================================================

interface DefaultProps {
  activeConversationId: string | null | undefined;
  storeContextKey: string;
  currentContextType: ContextType;
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
});
