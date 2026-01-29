/**
 * TaskChatPanel tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskChatPanel } from "./TaskChatPanel";

// Mock Tauri API
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Mock chat hooks
vi.mock("@/hooks/useChat", () => ({
  useChat: vi.fn(() => ({
    conversations: { data: [], isLoading: false },
    messages: { data: { messages: [] }, isLoading: false },
    sendMessage: { mutateAsync: vi.fn(), isPending: false },
    switchConversation: vi.fn(),
    createConversation: vi.fn(),
  })),
  chatKeys: {
    conversationList: vi.fn((type, id) => ["conversations", type, id]),
  },
}));

// Mock chat store
vi.mock("@/stores/chatStore", () => ({
  useChatStore: vi.fn((selector) => {
    const state = {
      queueMessage: vi.fn(),
      editQueuedMessage: vi.fn(),
      deleteQueuedMessage: vi.fn(),
      startEditingQueuedMessage: vi.fn(),
      activeConversationId: null,
    };
    return selector ? selector(state) : state;
  }),
  selectQueuedMessages: vi.fn(() => () => []),
  selectIsAgentRunning: vi.fn(() => () => false),
  selectActiveConversationId: vi.fn(() => null),
}));

// Mock chat API
vi.mock("@/api/chat", () => ({
  chatApi: {
    listConversations: vi.fn().mockResolvedValue([]),
  },
}));

describe("TaskChatPanel", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
  });

  it("renders without crashing", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <TaskChatPanel taskId="test-task-1" contextType="task" />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("task-chat-panel")).toBeInTheDocument();
  });

  it("shows empty state when no messages", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <TaskChatPanel taskId="test-task-1" contextType="task" />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("task-chat-empty")).toBeInTheDocument();
    expect(screen.getByText("Start a conversation")).toBeInTheDocument();
  });

  it("shows context indicator with 'Task' label for regular mode", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <TaskChatPanel taskId="test-task-1" contextType="task" />
      </QueryClientProvider>
    );

    expect(screen.getByText("Task")).toBeInTheDocument();
  });

  it("shows context indicator with 'Worker Execution' label for execution mode", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <TaskChatPanel taskId="test-task-1" contextType="task_execution" />
      </QueryClientProvider>
    );

    expect(screen.getByText("Worker Execution")).toBeInTheDocument();
  });

  it("shows worker executing indicator in execution mode", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <TaskChatPanel taskId="test-task-1" contextType="task_execution" />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("task-chat-worker-executing")).toBeInTheDocument();
    expect(screen.getByText("Worker is executing...")).toBeInTheDocument();
  });
});
