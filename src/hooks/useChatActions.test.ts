import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useChatActions } from "./useChatActions";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Mocks
// ============================================================================

const mockInvalidateQueries = vi.fn();
vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({ invalidateQueries: mockInvalidateQueries }),
}));

const mockActions = {
  queueMessage: vi.fn(),
  deleteQueuedMessage: vi.fn(),
  startEditingQueuedMessage: vi.fn(),
  setActiveConversation: vi.fn(),
  setAgentRunning: vi.fn(),
};
vi.mock("@/stores/chatStore", () => ({
  useChatStore: (selector: (state: typeof mockActions) => unknown) => selector(mockActions),
}));

const mockSendAgentMessage = vi.fn();
const mockQueueAgentMessage = vi.fn();
const mockDeleteQueuedAgentMessage = vi.fn();
const mockStopAgent = vi.fn();

vi.mock("@/api/chat", () => ({
  chatApi: {
    sendAgentMessage: (...args: unknown[]) => mockSendAgentMessage(...args),
    queueAgentMessage: (...args: unknown[]) => mockQueueAgentMessage(...args),
    deleteQueuedAgentMessage: (...args: unknown[]) => mockDeleteQueuedAgentMessage(...args),
  },
  stopAgent: (...args: unknown[]) => mockStopAgent(...args),
}));

const mockRecoverTaskExecution = vi.fn();
vi.mock("@/api/recovery", () => ({
  recoverTaskExecution: (...args: unknown[]) => mockRecoverTaskExecution(...args),
}));

const mockSpawnSessionNamer = vi.fn();
vi.mock("@/api/ideation", () => ({
  ideationApi: {
    sessions: {
      spawnSessionNamer: (...args: unknown[]) => mockSpawnSessionNamer(...args),
    },
  },
}));

vi.mock("@/hooks/useChat", () => ({
  chatKeys: {
    all: ["chat"] as const,
    conversations: () => ["chat", "conversations"] as const,
    conversation: (id: string) => ["chat", "conversations", id] as const,
    conversationList: (ct: string, ci: string) => ["chat", "conversations", ct, ci] as const,
  },
}));

// ============================================================================
// Helpers
// ============================================================================

interface SetupOptions {
  contextType?: ContextType;
  contextId?: string;
  storeContextKey?: string;
  selectedTaskId?: string | undefined;
  ideationSessionId?: string | undefined;
  isPending?: boolean;
  messageCount?: number;
}

function setup(opts: SetupOptions = {}) {
  const {
    contextType = "task",
    contextId = "task-1",
    storeContextKey = "task:task-1",
    selectedTaskId = undefined,
    ideationSessionId = undefined,
    isPending = false,
    messageCount = 5,
  } = opts;

  const mutateAsync = vi.fn().mockResolvedValue(undefined);

  const { result } = renderHook(() =>
    useChatActions({
      contextType,
      contextId,
      storeContextKey,
      selectedTaskId,
      ideationSessionId,
      sendMessage: { isPending, mutateAsync },
      messageCount,
    })
  );

  return { result, mutateAsync };
}

// ============================================================================
// Tests
// ============================================================================

describe("useChatActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSendAgentMessage.mockResolvedValue({
      conversationId: "conv-1",
      agentRunId: "run-1",
      isNewConversation: false,
    });
    mockQueueAgentMessage.mockResolvedValue({ id: "q-1" });
    mockDeleteQueuedAgentMessage.mockResolvedValue(true);
    mockStopAgent.mockResolvedValue(true);
    mockRecoverTaskExecution.mockResolvedValue(true);
    mockSpawnSessionNamer.mockResolvedValue(undefined);
  });

  // ── handleSend ──────────────────────────────────────────────────

  describe("handleSend", () => {
    it("calls sendMessage.mutateAsync with content", async () => {
      const { result, mutateAsync } = setup();

      await act(async () => {
        await result.current.handleSend("hello world");
      });

      expect(mutateAsync).toHaveBeenCalledWith({ content: "hello world", attachmentIds: undefined });
    });

    it("does not send empty or whitespace-only strings", async () => {
      const { result, mutateAsync } = setup();

      await act(async () => {
        await result.current.handleSend("");
        await result.current.handleSend("   ");
        await result.current.handleSend("\n\t");
      });

      expect(mutateAsync).not.toHaveBeenCalled();
    });

    it("does not send when isPending is true", async () => {
      const { result, mutateAsync } = setup({ isPending: true });

      await act(async () => {
        await result.current.handleSend("hello");
      });

      expect(mutateAsync).not.toHaveBeenCalled();
    });

    it("review mode sends via chatApi.sendAgentMessage directly", async () => {
      const { result, mutateAsync } = setup({
        contextType: "review",
        contextId: "task-42",
        storeContextKey: "review:task-42",
        selectedTaskId: "task-42",
      });

      await act(async () => {
        await result.current.handleSend("looks good");
      });

      // Should use direct API, NOT the mutation
      expect(mutateAsync).not.toHaveBeenCalled();
      expect(mockSendAgentMessage).toHaveBeenCalledWith("review", "task-42", "looks good", undefined, undefined);
      expect(mockActions.setAgentRunning).toHaveBeenCalledWith("review:task-42", true);
      expect(mockInvalidateQueries).toHaveBeenCalled();
    });

    it("review mode sets activeConversation when isNewConversation is true", async () => {
      mockSendAgentMessage.mockResolvedValue({
        conversationId: "new-conv",
        agentRunId: "run-1",
        isNewConversation: true,
      });

      const { result } = setup({
        contextType: "review",
        contextId: "task-42",
        storeContextKey: "review:task-42",
        selectedTaskId: "task-42",
      });

      await act(async () => {
        await result.current.handleSend("review this");
      });

      expect(mockActions.setActiveConversation).toHaveBeenCalledWith("new-conv");
    });

    it("ideation first message triggers auto-naming", async () => {
      const { result } = setup({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
        messageCount: 0,
      });

      await act(async () => {
        await result.current.handleSend("build a todo app");
      });

      expect(mockSpawnSessionNamer).toHaveBeenCalledWith("session-1", "build a todo app");
    });

    it("ideation does not trigger auto-naming when messageCount > 0", async () => {
      const { result } = setup({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
        messageCount: 3,
      });

      await act(async () => {
        await result.current.handleSend("follow-up message");
      });

      expect(mockSpawnSessionNamer).not.toHaveBeenCalled();
    });
  });

  // ── handleQueue ─────────────────────────────────────────────────

  describe("handleQueue", () => {
    it("adds to local store and sends to backend", async () => {
      const { result } = setup();

      await act(async () => {
        await result.current.handleQueue("queued msg");
      });

      // Local store should be updated with the content and a generated ID
      expect(mockActions.queueMessage).toHaveBeenCalledWith(
        "task:task-1",
        "queued msg",
        expect.stringMatching(/^queued-\d+-[a-z0-9]+$/)
      );

      // Backend should receive the same message
      expect(mockQueueAgentMessage).toHaveBeenCalledWith(
        "task",
        "task-1",
        "queued msg",
        expect.stringMatching(/^queued-\d+-[a-z0-9]+$/),
        undefined,
        undefined
      );
    });

    it("does not queue empty strings", async () => {
      const { result } = setup();

      await act(async () => {
        await result.current.handleQueue("  ");
      });

      expect(mockActions.queueMessage).not.toHaveBeenCalled();
      expect(mockQueueAgentMessage).not.toHaveBeenCalled();
    });
  });

  // ── handleStopAgent ─────────────────────────────────────────────

  describe("handleStopAgent", () => {
    it("calls stopAgent API", async () => {
      const { result } = setup({
        contextType: "ideation",
        contextId: "session-1",
      });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("ideation", "session-1");
      expect(mockRecoverTaskExecution).not.toHaveBeenCalled();
    });

    it("task_execution mode also calls recoverTaskExecution", async () => {
      const { result } = setup({
        contextType: "task_execution",
        contextId: "task-99",
        selectedTaskId: "task-99",
      });

      await act(async () => {
        await result.current.handleStopAgent();
      });

      expect(mockStopAgent).toHaveBeenCalledWith("task_execution", "task-99");
      expect(mockRecoverTaskExecution).toHaveBeenCalledWith("task-99");
    });
  });

  // ── handleDeleteQueuedMessage ───────────────────────────────────

  describe("handleDeleteQueuedMessage", () => {
    it("deletes from store and backend", async () => {
      const { result } = setup();

      await act(async () => {
        await result.current.handleDeleteQueuedMessage("msg-123");
      });

      expect(mockActions.deleteQueuedMessage).toHaveBeenCalledWith("task:task-1", "msg-123");
      expect(mockDeleteQueuedAgentMessage).toHaveBeenCalledWith("task", "task-1", "msg-123");
    });
  });

  // ── handleEditQueuedMessage ─────────────────────────────────────

  describe("handleEditQueuedMessage", () => {
    it("deletes old and creates new with fresh ID", async () => {
      const { result } = setup();

      await act(async () => {
        await result.current.handleEditQueuedMessage("old-id", "updated content");
      });

      // Old message deleted from backend and store
      expect(mockDeleteQueuedAgentMessage).toHaveBeenCalledWith("task", "task-1", "old-id");
      expect(mockActions.deleteQueuedMessage).toHaveBeenCalledWith("task:task-1", "old-id");

      // New message queued with fresh ID
      expect(mockActions.queueMessage).toHaveBeenCalledWith(
        "task:task-1",
        "updated content",
        expect.stringMatching(/^queued-\d+-[a-z0-9]+$/)
      );
      expect(mockQueueAgentMessage).toHaveBeenCalledWith(
        "task",
        "task-1",
        "updated content",
        expect.stringMatching(/^queued-\d+-[a-z0-9]+$/)
      );

      // The new ID should be different from the old
      const newId = mockActions.queueMessage.mock.calls[0][2];
      expect(newId).not.toBe("old-id");
    });
  });

  // ── handleEditLastQueued ────────────────────────────────────────

  describe("handleEditLastQueued", () => {
    it("starts editing last queued message", () => {
      const { result } = setup();

      act(() => {
        result.current.handleEditLastQueued([
          { id: "q-1" },
          { id: "q-2" },
          { id: "q-3" },
        ]);
      });

      expect(mockActions.startEditingQueuedMessage).toHaveBeenCalledWith("task:task-1", "q-3");
    });

    it("does nothing when queue is empty", () => {
      const { result } = setup();

      act(() => {
        result.current.handleEditLastQueued([]);
      });

      expect(mockActions.startEditingQueuedMessage).not.toHaveBeenCalled();
    });
  });
});
