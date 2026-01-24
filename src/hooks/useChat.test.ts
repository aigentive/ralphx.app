/**
 * useChat hook tests
 *
 * Tests for useChat and useChatMessages hooks
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useChat, useChatMessages, chatKeys } from "./useChat";
import { chatApi } from "@/api/chat";
import type { ChatMessageResponse } from "@/api/chat";
import type { ChatContext } from "@/types/chat";

// Mock the chat API
vi.mock("@/api/chat", () => ({
  chatApi: {
    sendMessageWithContext: vi.fn(),
    getSessionMessages: vi.fn(),
    getProjectMessages: vi.fn(),
    getTaskMessages: vi.fn(),
  },
}));

// Create mock data
const mockMessage1: ChatMessageResponse = {
  id: "message-1",
  sessionId: "session-1",
  projectId: null,
  taskId: null,
  role: "user",
  content: "Hello",
  metadata: null,
  parentMessageId: null,
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
  createdAt: "2026-01-24T10:00:05Z",
};

const mockProjectMessage: ChatMessageResponse = {
  id: "message-3",
  sessionId: null,
  projectId: "project-1",
  taskId: null,
  role: "user",
  content: "Project question",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T10:05:00Z",
};

const mockTaskMessage: ChatMessageResponse = {
  id: "message-4",
  sessionId: null,
  projectId: null,
  taskId: "task-1",
  role: "user",
  content: "Task question",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T10:10:00Z",
};

// Test contexts
const ideationContext: ChatContext = {
  view: "ideation",
  projectId: "project-1",
  ideationSessionId: "session-1",
};

const kanbanContext: ChatContext = {
  view: "kanban",
  projectId: "project-1",
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

describe("chatKeys", () => {
  it("should generate correct key for all", () => {
    expect(chatKeys.all).toEqual(["chat"]);
  });

  it("should generate correct key for messages", () => {
    expect(chatKeys.messages()).toEqual(["chat", "messages"]);
  });

  it("should generate correct key for session messages", () => {
    expect(chatKeys.sessionMessages("session-1")).toEqual([
      "chat",
      "messages",
      "session",
      "session-1",
    ]);
  });

  it("should generate correct key for project messages", () => {
    expect(chatKeys.projectMessages("project-1")).toEqual([
      "chat",
      "messages",
      "project",
      "project-1",
    ]);
  });

  it("should generate correct key for task messages", () => {
    expect(chatKeys.taskMessages("task-1")).toEqual([
      "chat",
      "messages",
      "task",
      "task-1",
    ]);
  });
});

describe("useChatMessages", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch session messages for ideation context", async () => {
    const mockMessages = [mockMessage1, mockMessage2];
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce(mockMessages);

    const { result } = renderHook(() => useChatMessages(ideationContext), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockMessages);
    expect(chatApi.getSessionMessages).toHaveBeenCalledWith("session-1");
  });

  it("should fetch project messages for kanban context", async () => {
    const mockMessages = [mockProjectMessage];
    vi.mocked(chatApi.getProjectMessages).mockResolvedValueOnce(mockMessages);

    const { result } = renderHook(() => useChatMessages(kanbanContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockMessages);
    expect(chatApi.getProjectMessages).toHaveBeenCalledWith("project-1");
  });

  it("should fetch task messages for task detail context", async () => {
    const mockMessages = [mockTaskMessage];
    vi.mocked(chatApi.getTaskMessages).mockResolvedValueOnce(mockMessages);

    const { result } = renderHook(() => useChatMessages(taskDetailContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockMessages);
    expect(chatApi.getTaskMessages).toHaveBeenCalledWith("task-1");
  });

  it("should return empty array when no messages exist", async () => {
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChatMessages(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([]);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch messages");
    vi.mocked(chatApi.getSessionMessages).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useChatMessages(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useChat", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should send message in ideation context", async () => {
    const newMessage: ChatMessageResponse = {
      ...mockMessage1,
      id: "new-message",
      content: "New message content",
    };
    vi.mocked(chatApi.sendMessageWithContext).mockResolvedValueOnce(newMessage);
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.sendMessage.mutateAsync("New message content");
    });

    expect(chatApi.sendMessageWithContext).toHaveBeenCalledWith(
      ideationContext,
      "New message content",
      undefined
    );
  });

  it("should send message in kanban context", async () => {
    const newMessage: ChatMessageResponse = {
      ...mockProjectMessage,
      id: "new-message",
      content: "Kanban message",
    };
    vi.mocked(chatApi.sendMessageWithContext).mockResolvedValueOnce(newMessage);
    vi.mocked(chatApi.getProjectMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChat(kanbanContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.sendMessage.mutateAsync("Kanban message");
    });

    expect(chatApi.sendMessageWithContext).toHaveBeenCalledWith(
      kanbanContext,
      "Kanban message",
      undefined
    );
  });

  it("should send message with options", async () => {
    const newMessage: ChatMessageResponse = {
      ...mockMessage1,
      id: "new-message",
      parentMessageId: "message-1",
    };
    vi.mocked(chatApi.sendMessageWithContext).mockResolvedValueOnce(newMessage);
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.sendMessage.mutateAsync({
        content: "Reply message",
        parentMessageId: "message-1",
      });
    });

    expect(chatApi.sendMessageWithContext).toHaveBeenCalledWith(
      ideationContext,
      "Reply message",
      { parentMessageId: "message-1" }
    );
  });

  it("should handle send message error", async () => {
    const error = new Error("Failed to send message");
    vi.mocked(chatApi.sendMessageWithContext).mockRejectedValueOnce(error);
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.sendMessage.mutateAsync("Message");
      })
    ).rejects.toThrow("Failed to send message");
  });

  it("should provide messages from context", async () => {
    const mockMessages = [mockMessage1, mockMessage2];
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce(mockMessages);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.messages.isSuccess).toBe(true));

    expect(result.current.messages.data).toEqual(mockMessages);
  });

  it("should return isLoading based on messages state", async () => {
    vi.mocked(chatApi.getSessionMessages).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useChat(ideationContext), {
      wrapper: createWrapper(),
    });

    // Initially loading
    expect(result.current.messages.isLoading).toBe(true);

    await waitFor(() => expect(result.current.messages.isLoading).toBe(false));
  });
});
