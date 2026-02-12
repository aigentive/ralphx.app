/**
 * Tests for useIntegratedChatHandlers hook
 *
 * Covers:
 * - Stop context resolution for all chat modes (ideation, review, merge, task, project, execution)
 * - Stop sequencing (stopAgent first, then recoverTaskExecution for execution mode)
 * - Graceful handling when stop fails or no agent is running
 * - Queue context resolution
 */

import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { useIntegratedChatHandlers } from "./useIntegratedChatHandlers";
import { createWrapper } from "@/test/store-utils";
import { useChatStore } from "@/stores/chatStore";

// Mock API modules
vi.mock("@/api/chat", () => ({
  chatApi: {
    sendAgentMessage: vi.fn().mockResolvedValue({ conversationId: "conv-1", isNewConversation: false }),
    queueAgentMessage: vi.fn().mockResolvedValue(undefined),
    deleteQueuedAgentMessage: vi.fn().mockResolvedValue(undefined),
    listConversations: vi.fn().mockResolvedValue([]),
  },
  stopAgent: vi.fn().mockResolvedValue(true),
}));

vi.mock("@/api/recovery", () => ({
  recoverTaskExecution: vi.fn().mockResolvedValue(true),
}));

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    sessions: {
      spawnSessionNamer: vi.fn().mockResolvedValue(undefined),
    },
  },
}));

// Import mocked modules for assertions
import { stopAgent } from "@/api/chat";
import { recoverTaskExecution } from "@/api/recovery";

const mockStopAgent = stopAgent as ReturnType<typeof vi.fn>;
const mockRecoverTaskExecution = recoverTaskExecution as ReturnType<typeof vi.fn>;

// Default props factory
function createDefaultProps(overrides: Partial<Parameters<typeof useIntegratedChatHandlers>[0]> = {}) {
  return {
    isExecutionMode: false,
    isReviewMode: false,
    isMergeMode: false,
    selectedTaskId: "task-123",
    projectId: "project-456",
    ideationSessionId: undefined as string | undefined,
    storeContextKey: "task:task-123",
    sendMessage: {
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue(undefined),
    },
    messageCount: 0,
    ...overrides,
  };
}

describe("useIntegratedChatHandlers", () => {
  const wrapper = createWrapper();

  beforeEach(() => {
    vi.clearAllMocks();
    useChatStore.setState({
      messages: {},
      context: null,
      width: 320,
      isLoading: false,
      activeConversationId: null,
      queuedMessages: {},
      isAgentRunning: {},
      isSending: {},
    });
  });

  describe("handleStopAgent - context resolution", () => {
    it("stops with 'review' context in review mode", async () => {
      const props = createDefaultProps({
        isReviewMode: true,
        storeContextKey: "review:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("review", "task-123");
    });

    it("stops with 'ideation' context in ideation mode", async () => {
      const props = createDefaultProps({
        ideationSessionId: "session-789",
        selectedTaskId: undefined,
        storeContextKey: "session:session-789",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("ideation", "session-789");
    });

    it("stops with 'task' context for regular task chat", async () => {
      const props = createDefaultProps();

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("task", "task-123");
    });

    it("stops with 'project' context when no task or session selected", async () => {
      const props = createDefaultProps({
        selectedTaskId: undefined,
        storeContextKey: "project:project-456",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("project", "project-456");
    });

    it("in execution mode, calls stopAgent first then recoverTaskExecution", async () => {
      const props = createDefaultProps({
        isExecutionMode: true,
        storeContextKey: "task_execution:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      // Both should be called: stopAgent for immediate cancellation, then recovery
      expect(mockStopAgent).toHaveBeenCalledWith("task_execution", "task-123");
      expect(mockRecoverTaskExecution).toHaveBeenCalledWith("task-123");
    });

    it("execution mode stop does not throw on failure", async () => {
      mockRecoverTaskExecution.mockRejectedValueOnce(new Error("Network error"));

      const props = createDefaultProps({
        isExecutionMode: true,
        storeContextKey: "task_execution:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      // Should not throw
      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockRecoverTaskExecution).toHaveBeenCalledWith("task-123");
    });

    it("stopAgent does not throw on failure", async () => {
      mockStopAgent.mockRejectedValueOnce(new Error("Network error"));

      const props = createDefaultProps();

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      // Should not throw
      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("task", "task-123");
    });
  });

  describe("handleStopAgent - merge and negative cases", () => {
    it("stops with 'merge' context in merge mode", async () => {
      const props = createDefaultProps({
        isMergeMode: true,
        storeContextKey: "merge:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("merge", "task-123");
      // Merge mode should NOT call recoverTaskExecution
      expect(mockRecoverTaskExecution).not.toHaveBeenCalled();
    });

    it("handles stop gracefully when no agent is running (negative case)", async () => {
      // stopAgent returns successfully even if no agent run is active
      mockStopAgent.mockResolvedValueOnce(true);

      const props = createDefaultProps();

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      // Should not throw even when there's no running agent
      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("task", "task-123");
      // No recoverTaskExecution in non-execution mode
      expect(mockRecoverTaskExecution).not.toHaveBeenCalled();
    });
  });

  describe("getQueueContext - context resolution", () => {
    it("resolves to 'task_execution' context in execution mode", async () => {
      const mockQueueApi = (await import("@/api/chat")).chatApi.queueAgentMessage as ReturnType<typeof vi.fn>;

      const props = createDefaultProps({
        isExecutionMode: true,
        storeContextKey: "task_execution:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleQueue("test message");
      });

      expect(mockQueueApi).toHaveBeenCalledWith(
        "task_execution",
        "task-123",
        "test message",
        expect.stringContaining("queued-")
      );
    });

    it("resolves to 'review' context in review mode", async () => {
      const mockQueueApi = (await import("@/api/chat")).chatApi.queueAgentMessage as ReturnType<typeof vi.fn>;

      const props = createDefaultProps({
        isReviewMode: true,
        storeContextKey: "review:task-123",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleQueue("test message");
      });

      expect(mockQueueApi).toHaveBeenCalledWith(
        "review",
        "task-123",
        "test message",
        expect.stringContaining("queued-")
      );
    });

    it("resolves to 'ideation' context in ideation mode", async () => {
      const mockQueueApi = (await import("@/api/chat")).chatApi.queueAgentMessage as ReturnType<typeof vi.fn>;

      const props = createDefaultProps({
        ideationSessionId: "session-789",
        selectedTaskId: undefined,
        storeContextKey: "session:session-789",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleQueue("test message");
      });

      expect(mockQueueApi).toHaveBeenCalledWith(
        "ideation",
        "session-789",
        "test message",
        expect.stringContaining("queued-")
      );
    });

    it("resolves to 'project' context when no task or session", async () => {
      const mockQueueApi = (await import("@/api/chat")).chatApi.queueAgentMessage as ReturnType<typeof vi.fn>;

      const props = createDefaultProps({
        selectedTaskId: undefined,
        storeContextKey: "project:project-456",
      });

      const { result } = renderHook(() => useIntegratedChatHandlers(props), { wrapper });

      await act(async () => {
        await result.current.handleQueue("test message");
      });

      expect(mockQueueApi).toHaveBeenCalledWith(
        "project",
        "project-456",
        "test message",
        expect.stringContaining("queued-")
      );
    });
  });
});
