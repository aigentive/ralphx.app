/**
 * useChat hook tests
 *
 * Tests for useChat, useConversations, and useAgentRunStatus hooks
 * using TanStack Query with mocked API and Tauri events.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useChat,
  useConversations,
  useConversation,
  useAgentRunStatus,
  chatKeys,
} from "./useChat";
import { chatApi } from "@/api/chat";
import type { ChatMessageResponse } from "@/api/chat";
import type { ChatContext } from "@/types/chat";
import type { ChatConversation, AgentRun } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";

// Mock Tauri event listener
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// Mock chat store
vi.mock("@/stores/chatStore", () => ({
  useChatStore: vi.fn(),
}));

// Mock ideation keys
vi.mock("./useIdeation", () => ({
  ideationKeys: {
    sessionWithData: vi.fn((id: string) => ["ideation", "session", id, "data"]),
  },
}));

// Mock the chat API
vi.mock("@/api/chat", () => ({
  chatApi: {
    sendContextMessage: vi.fn(),
    listConversations: vi.fn(),
    getConversation: vi.fn(),
    createConversation: vi.fn(),
    getAgentRunStatus: vi.fn(),
  },
}));

// Create mock data
const mockConversation1: ChatConversation = {
  id: "conv-1",
  contextType: "ideation",
  contextId: "session-1",
  claudeSessionId: "claude-session-1",
  title: "First conversation",
  messageCount: 2,
  lastMessageAt: "2026-01-24T10:00:00Z",
  createdAt: "2026-01-24T09:00:00Z",
  updatedAt: "2026-01-24T10:00:00Z",
};

const mockConversation2: ChatConversation = {
  id: "conv-2",
  contextType: "ideation",
  contextId: "session-1",
  claudeSessionId: null,
  title: "Second conversation",
  messageCount: 1,
  lastMessageAt: "2026-01-24T11:00:00Z",
  createdAt: "2026-01-24T11:00:00Z",
  updatedAt: "2026-01-24T11:00:00Z",
};

const mockMessage1: ChatMessageResponse = {
  id: "message-1",
  sessionId: "session-1",
  projectId: null,
  taskId: null,
  role: "user",
  content: "Hello",
  metadata: null,
  parentMessageId: null,
  conversationId: "conv-1",
  toolCalls: null,
  createdAt: "2026-01-24T10:00:00Z",
};

const mockMessage2: ChatMessageResponse = {
  id: "message-2",
  sessionId: "session-1",
  projectId: null,
  taskId: null,
  role: "orchestrator",
  content: "Hi there! How can I help?",
  metadata: null,
  parentMessageId: "message-1",
  conversationId: "conv-1",
  toolCalls: null,
  createdAt: "2026-01-24T10:00:05Z",
};

const mockAgentRun: AgentRun = {
  id: "run-1",
  conversationId: "conv-1",
  status: "running",
  startedAt: "2026-01-24T10:00:10Z",
  completedAt: null,
  errorMessage: null,
};

// Test contexts
const ideationContext: ChatContext = {
  view: "ideation",
  projectId: "project-1",
  ideationSessionId: "session-1",
};

const taskDetailContext: ChatContext = {
  view: "task_detail",
  projectId: "project-1",
  selectedTaskId: "task-1",
};

// Test wrapper with QueryClientProvider
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

// Mock chat store state
const mockStoreState = {
  activeConversationId: null as string | null,
  setActiveConversation: vi.fn(),
  setAgentRunning: vi.fn(),
  queuedMessages: [],
  processQueue: vi.fn(),
};

describe("chatKeys", () => {
  it("should generate correct key for conversations", () => {
    expect(chatKeys.conversations()).toEqual(["chat", "conversations"]);
  });

  it("should generate correct key for conversation", () => {
    expect(chatKeys.conversation("conv-1")).toEqual([
      "chat",
      "conversations",
      "conv-1",
    ]);
  });

  it("should generate correct key for conversation list", () => {
    expect(chatKeys.conversationList("ideation", "session-1")).toEqual([
      "chat",
      "conversations",
      "ideation",
      "session-1",
    ]);
  });

  it("should generate correct key for agent run", () => {
    expect(chatKeys.agentRun("conv-1")).toEqual([
      "chat",
      "agent-run",
      "conv-1",
    ]);
  });
});

describe("useConversations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch conversations for ideation context", async () => {
    const mockConversations = [mockConversation1, mockConversation2];
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce(
      mockConversations
    );

    const { result } = renderHook(() => useConversations(ideationContext), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockConversations);
    expect(chatApi.listConversations).toHaveBeenCalledWith(
      "ideation",
      "session-1"
    );
  });

  it("should fetch conversations for task context", async () => {
    const mockConversations = [mockConversation1];
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce(
      mockConversations
    );

    const { result } = renderHook(() => useConversations(taskDetailContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockConversations);
    expect(chatApi.listConversations).toHaveBeenCalledWith("task", "task-1");
  });
});

describe("useConversation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch conversation with messages", async () => {
    const mockData = {
      conversation: mockConversation1,
      messages: [mockMessage1, mockMessage2],
    };
    vi.mocked(chatApi.getConversation).mockResolvedValueOnce(mockData);

    const { result } = renderHook(() => useConversation("conv-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockData);
    expect(chatApi.getConversation).toHaveBeenCalledWith("conv-1");
  });

  it("should not fetch when conversationId is null", async () => {
    const { result } = renderHook(() => useConversation(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(chatApi.getConversation).not.toHaveBeenCalled();
  });
});

describe("useAgentRunStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch agent run status", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(mockAgentRun);

    const { result } = renderHook(() => useAgentRunStatus("conv-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockAgentRun);
    expect(chatApi.getAgentRunStatus).toHaveBeenCalledWith("conv-1");
  });

  it("should return null when no agent is running", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useAgentRunStatus("conv-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("should not fetch when conversationId is null", async () => {
    const { result } = renderHook(() => useAgentRunStatus(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(chatApi.getAgentRunStatus).not.toHaveBeenCalled();
  });
});

describe("useChat", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock store state
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    vi.mocked(useChatStore).mockImplementation((selector?: any) => {
      if (typeof selector === "function") {
        return selector(mockStoreState);
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return mockStoreState as any;
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should send context-aware message", async () => {
    // sendContextMessage now returns SendContextMessageResult
    const mockResult = {
      responseText: "AI response",
      toolCalls: [],
      claudeSessionId: "claude-session-123",
      conversationId: "conv-1",
    };
    vi.mocked(chatApi.sendContextMessage).mockResolvedValueOnce(mockResult);
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.sendMessage.mutateAsync("New message content");
    });

    expect(chatApi.sendContextMessage).toHaveBeenCalledWith(
      "ideation",
      "session-1",
      "New message content"
    );
  });

  it("should send message in task context", async () => {
    // sendContextMessage now returns SendContextMessageResult
    const mockResult = {
      responseText: "AI response for task",
      toolCalls: [],
      claudeSessionId: "claude-session-456",
      conversationId: "conv-2",
    };
    vi.mocked(chatApi.sendContextMessage).mockResolvedValueOnce(mockResult);
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useChat(taskDetailContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.sendMessage.mutateAsync("Task message");
    });

    expect(chatApi.sendContextMessage).toHaveBeenCalledWith(
      "task",
      "task-1",
      "Task message"
    );
  });

  it("should create new conversation", async () => {
    vi.mocked(chatApi.createConversation).mockResolvedValueOnce(
      mockConversation1
    );
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.createConversation();
    });

    expect(chatApi.createConversation).toHaveBeenCalledWith(
      "ideation",
      "session-1"
    );
    expect(mockStoreState.setActiveConversation).toHaveBeenCalledWith("conv-1");
  });

  it("should switch conversation", async () => {
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      result.current.switchConversation("conv-2");
    });

    expect(mockStoreState.setActiveConversation).toHaveBeenCalledWith("conv-2");
  });

  it("should update agent running state from agent run status", async () => {
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(mockAgentRun);

    // Set active conversation in store
    const storeWithConversation = {
      ...mockStoreState,
      activeConversationId: "conv-1" as string | null,
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    vi.mocked(useChatStore).mockImplementation((selector?: any) => {
      if (typeof selector === "function") {
        return selector(storeWithConversation);
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return storeWithConversation as any;
    });

    renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(mockStoreState.setAgentRunning).toHaveBeenCalledWith(true);
    });
  });

  it("should initialize active conversation from conversations list", async () => {
    const mockConversations = [mockConversation1, mockConversation2];
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce(
      mockConversations
    );
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      // Should set the most recent conversation (conv-2 has later lastMessageAt)
      expect(mockStoreState.setActiveConversation).toHaveBeenCalledWith(
        "conv-2"
      );
    });
  });

  it("should provide conversations, activeConversation, and agentRunStatus", async () => {
    const mockConversations = [mockConversation1];
    const mockConversationData = {
      conversation: mockConversation1,
      messages: [mockMessage1, mockMessage2],
    };

    vi.mocked(chatApi.listConversations).mockResolvedValueOnce(
      mockConversations
    );
    vi.mocked(chatApi.getConversation).mockResolvedValueOnce(
      mockConversationData
    );
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(mockAgentRun);

    // Set active conversation in store
    const storeWithConversation = {
      ...mockStoreState,
      activeConversationId: "conv-1" as string | null,
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    vi.mocked(useChatStore).mockImplementation((selector?: any) => {
      if (typeof selector === "function") {
        return selector(storeWithConversation);
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return storeWithConversation as any;
    });

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.conversations.isSuccess).toBe(true);
    });

    expect(result.current.conversations.data).toEqual(mockConversations);

    await waitFor(() => {
      expect(result.current.activeConversation.isSuccess).toBe(true);
    });

    expect(result.current.activeConversation.data).toEqual(
      mockConversationData
    );

    await waitFor(() => {
      expect(result.current.agentRunStatus.isSuccess).toBe(true);
    });

    expect(result.current.agentRunStatus.data).toEqual(mockAgentRun);
  });

  it("should handle send message error", async () => {
    const error = new Error("Failed to send message");
    vi.mocked(chatApi.sendContextMessage).mockRejectedValueOnce(error);
    vi.mocked(chatApi.listConversations).mockResolvedValueOnce([]);
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.sendMessage.mutateAsync("Message");
      })
    ).rejects.toThrow("Failed to send message");
  });
});
