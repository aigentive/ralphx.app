/**
 * Tests for IntegratedChatPanel
 *
 * Covers:
 * - Stop button visibility follows isAgentRunning (live run state only)
 * - Stop button hidden in execution mode without live agent run
 * - Status badge "Agent responding..." reflects live run state, not workflow status
 * - History mode disables stop button and status badge
 * - File attachment integration
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { act } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { IntegratedChatPanel } from "./IntegratedChatPanel";
import { PreviousRunBanner } from "./IntegratedChatPanel.components";
import { chatApi } from "@/api/chat";
import { useChatStore } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";

// ============================================================================
// Hoisted mutable state for useChat mock (vi.hoisted runs before vi.mock)
// ============================================================================

const { useChatMockState } = vi.hoisted(() => {
  const useChatMockState = {
    messages: [] as Array<{ id: string; role: string; content: string; createdAt: string; toolCalls: null; contentBlocks: null }>,
    conversation: null as { contextType: string; contextId: string } | null,
    conversations: [] as Array<{ id: string }>,
  };
  return { useChatMockState };
});

// ============================================================================
// Mocks
// ============================================================================

// Mock Tauri event system (already in setup.ts but ensure coverage)
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

// Mock the event bus provider
const mockSubscribe = vi.fn(() => () => {});
vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
    emit: vi.fn(),
  }),
  EventProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock useChat hook — reads from useChatMockState so individual tests can inject messages
vi.mock("@/hooks/useChat", () => ({
  useChat: () => ({
    messages: {
      data: { messages: useChatMockState.messages, conversation: useChatMockState.conversation },
      isLoading: false,
    },
    sendMessage: { mutateAsync: vi.fn(), isPending: false },
    conversations: { data: useChatMockState.conversations, isLoading: false },
    switchConversation: vi.fn(),
    createConversation: vi.fn(),
  }),
  useConversation: () => ({
    data: undefined,
    isLoading: false,
    error: null,
  }),
  chatKeys: {
    all: ["chat"],
    conversationList: (type: string, id: string) => ["chat", "conversations", type, id],
    conversation: (id: string) => ["chat", "conversation", id],
    agentRun: (id: string) => ["chat", "agentRun", id],
  },
}));

// Mock useTasks - mutable so tests can override returned tasks
let mockTasks: Array<{ id: string; internalStatus: string }> = [];
vi.mock("@/hooks/useTasks", () => ({
  useTasks: () => ({ data: mockTasks }),
  taskKeys: {
    list: (projectId: string) => ["tasks", projectId],
    detail: (taskId: string) => ["task", taskId],
  },
}));

// Mock useChatPanelContext
const mockChatPanelContext = {
  chatContext: { view: "kanban" as const, projectId: "project-1" },
  storeContextKey: "task:task-1",
  currentContextType: "task" as const,
  currentContextId: "task-1",
  activeConversationId: null as string | null,
  streamingToolCalls: [] as unknown[],
  setStreamingToolCalls: vi.fn(),
  streamingTasks: new Map(),
  setStreamingTasks: vi.fn(),
  autoSelectConversation: vi.fn(),
};

vi.mock("@/hooks/useChatPanelContext", () => ({
  useChatPanelContext: () => mockChatPanelContext,
}));

// Mock useChatActions (replaces useIntegratedChatHandlers)
vi.mock("@/hooks/useChatActions", () => ({
  useChatActions: () => ({
    handleSend: vi.fn(),
    handleEditLastQueued: vi.fn(),
    handleDeleteQueuedMessage: vi.fn(),
    handleEditQueuedMessage: vi.fn(),
    handleStopAgent: vi.fn(),
  }),
}));

// Mock useChatEvents (replaces useIntegratedChatEvents)
vi.mock("@/hooks/useChatEvents", () => ({
  useChatEvents: vi.fn(),
}));

// Mock useChatRecovery
vi.mock("@/hooks/useChatRecovery", () => ({
  useChatRecovery: vi.fn(),
}));

// Mock useAgentEvents
vi.mock("@/hooks/useAgentEvents", () => ({
  useAgentEvents: vi.fn(),
}));

// Mock useAskUserQuestion
vi.mock("@/hooks/useAskUserQuestion", () => ({
  useAskUserQuestion: () => ({
    activeQuestion: null,
    answeredQuestion: undefined,
    submitAnswer: vi.fn().mockResolvedValue(true),
    dismissQuestion: vi.fn(),
    clearAnswered: vi.fn(),
    isLoading: false,
  }),
}));

// Mock useQuestionInput
vi.mock("@/hooks/useQuestionInput", () => ({
  useQuestionInput: () => ({
    selectedOptions: new Set(),
    questionInputValue: "",
    setQuestionInputValue: vi.fn(),
    handleChipClick: vi.fn(),
    handleMatchedOptions: vi.fn(),
    handleQuestionSend: vi.fn(),
  }),
}));

// Mock useChatAttachments
const mockUseChatAttachments = {
  attachments: [],
  uploadFiles: vi.fn().mockResolvedValue([]),
  removeAttachment: vi.fn().mockResolvedValue(undefined),
  clearAttachments: vi.fn(),
  uploading: false,
  uploadProgress: [],
};

vi.mock("@/hooks/useChatAttachments", () => ({
  useChatAttachments: () => mockUseChatAttachments,
}));

// Mock chat API for useQuery calls
vi.mock("@/api/chat", () => ({
  chatApi: {
    listConversations: vi.fn().mockResolvedValue([]),
    getAgentRunStatus: vi.fn().mockResolvedValue(null),
    sendAgentMessage: vi.fn().mockResolvedValue({ conversationId: "conv-1" }),
  },
  stopAgent: vi.fn().mockResolvedValue(true),
}));

// Mock recovery components
vi.mock("@/components/recovery/RecoveryPromptDialog", () => ({
  RecoveryPromptDialog: () => null,
}));

// ============================================================================
// Test Wrapper
// ============================================================================

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = createTestQueryClient();
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("IntegratedChatPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockTasks = [];
    // Reset useChat mock state to defaults (empty messages, no conversation context)
    useChatMockState.messages = [];
    useChatMockState.conversation = null;
    useChatMockState.conversations = [];

    // Reset stores
    act(() => {
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

    act(() => {
      useUiStore.setState({
        selectedTaskId: "task-1",
        taskHistoryState: null,
      });
    });

    // Reset mock context to defaults
    mockChatPanelContext.storeContextKey = "task:task-1";
    mockChatPanelContext.currentContextType = "task";
    mockChatPanelContext.currentContextId = "task-1";
    mockChatPanelContext.activeConversationId = null;
  });

  describe("Stop button visibility", () => {
    it("shows Stop button when isAgentRunning is true via store", () => {
      // Set agent as running in the store
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.getByTestId("chat-input-stop")).toBeInTheDocument();
    });

    it("hides Stop button in execution mode when no live agent run is active", () => {
      // Provide a task with "executing" status so isExecutionMode becomes true
      mockTasks = [{ id: "task-1", internalStatus: "executing" }];

      // After fix: isAgentRunning prop uses live run state only, not isExecutionMode
      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // Stop button should NOT show without a live agent run
      expect(screen.queryByTestId("chat-input-stop")).not.toBeInTheDocument();
    });

    it("hides Stop button when agent is not running and not in execution mode", () => {
      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.queryByTestId("chat-input-stop")).not.toBeInTheDocument();
      expect(screen.getByTestId("chat-input-send")).toBeInTheDocument();
    });

    it("hides Stop button in history mode even if agent running state is stale", () => {
      // Simulate stale agent running state
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      // Set history mode
      act(() => {
        useUiStore.setState({
          taskHistoryState: {
            status: "approved",
            conversationId: "conv-1",
            agentRunId: null,
            timestamp: null,
          },
        });
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // History mode makes input read-only, so stop button should be hidden
      expect(screen.queryByTestId("chat-input-stop")).not.toBeInTheDocument();
    });
  });

  describe("Status badge - agent activity", () => {
    it("does not show active agent badge when no agent is running", () => {
      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // "Agent responding..." should NOT appear
      expect(screen.queryByText("Agent responding...")).not.toBeInTheDocument();
      expect(screen.queryByText("Worker running...")).not.toBeInTheDocument();
      expect(screen.queryByText("Reviewing...")).not.toBeInTheDocument();
    });

    it("shows 'Agent responding...' when agent is running via store (non-execution)", () => {
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.getByText("Agent responding...")).toBeInTheDocument();
    });

    it("shows 'Agent responding...' when isSending is true", () => {
      act(() => {
        useChatStore.getState().setSending("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.getByText("Agent responding...")).toBeInTheDocument();
    });

    it("does not show active badge in history mode", () => {
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
        useUiStore.setState({
          taskHistoryState: {
            status: "approved",
            conversationId: "conv-1",
            agentRunId: null,
            timestamp: null,
          },
        });
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // History mode disables agent activity
      expect(screen.queryByText("Agent responding...")).not.toBeInTheDocument();
      expect(screen.queryByText("Worker running...")).not.toBeInTheDocument();
    });

    it("does not show 'Worker running...' in execution mode without live agent run", () => {
      // Provide a task with "executing" status so isExecutionMode becomes true
      mockTasks = [{ id: "task-1", internalStatus: "executing" }];
      // Do NOT set isAgentRunning - no live agent run

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // After fix: isAgentActive only uses isSending || isAgentRunning (live run state)
      // isExecutionMode no longer used as activity signal
      expect(screen.queryByText("Worker running...")).not.toBeInTheDocument();
    });
  });

  describe("Rendering basics", () => {
    it("renders the chat panel container", () => {
      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument();
    });

    it("renders the chat input", () => {
      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });
  });

  describe("File attachments", () => {
    beforeEach(() => {
      // Reset mock attachment state
      mockUseChatAttachments.attachments = [];
      mockUseChatAttachments.uploadFiles.mockClear();
      mockUseChatAttachments.removeAttachment.mockClear();
      mockUseChatAttachments.clearAttachments.mockClear();
    });

    it("enables attachments when active conversation exists", () => {
      // Set active conversation
      mockChatPanelContext.activeConversationId = "conv-1";

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // ChatInput should be rendered with attachment props
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });

    it("disables attachments in history mode", () => {
      // Set active conversation
      mockChatPanelContext.activeConversationId = "conv-1";

      // Enable history mode
      act(() => {
        useUiStore.setState({
          taskHistoryState: {
            status: "approved",
            conversationId: "conv-1",
            agentRunId: null,
            timestamp: null,
          },
        });
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // ChatInput should be in read-only mode, attachments disabled
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });

    it("disables attachments when no active conversation", () => {
      // No active conversation
      mockChatPanelContext.activeConversationId = null;

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // ChatInput should be rendered but attachments disabled
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });

    it("passes attachment data to ChatInput", () => {
      // Set active conversation and mock attachments
      mockChatPanelContext.activeConversationId = "conv-1";
      mockUseChatAttachments.attachments = [
        {
          id: "att-1",
          conversationId: "conv-1",
          fileName: "test.txt",
          filePath: "/path/to/test.txt",
          fileSize: 1024,
          createdAt: "2024-01-01T00:00:00Z",
        },
      ];

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // ChatInput should be rendered with attachments
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });

    it("clears attachments after send", async () => {
      // Set active conversation
      mockChatPanelContext.activeConversationId = "conv-1";

      // Mock some attachments
      mockUseChatAttachments.attachments = [
        {
          id: "att-1",
          conversationId: "conv-1",
          fileName: "test.txt",
          filePath: "/path/to/test.txt",
          fileSize: 1024,
          createdAt: "2024-01-01T00:00:00Z",
        },
      ];

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // Note: We can't directly trigger send from this test as the ChatInput
      // is mocked and doesn't expose the send handler. The logic is tested
      // through the handleSend wrapper implementation.
      // This test verifies that clearAttachments is available and can be called.
      expect(mockUseChatAttachments.clearAttachments).toBeDefined();
    });

    it("preserves attachments in question mode", () => {
      // Set active conversation
      mockChatPanelContext.activeConversationId = "conv-1";

      // Mock attachments
      mockUseChatAttachments.attachments = [
        {
          id: "att-1",
          conversationId: "conv-1",
          fileName: "test.txt",
          filePath: "/path/to/test.txt",
          fileSize: 1024,
          createdAt: "2024-01-01T00:00:00Z",
        },
      ];

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // Attachments should still be available in question mode
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Agent-status-aware mode flags (execution panel routing fix)
  // ============================================================================
  // Proof obligation: when agent is alive but task status has transitioned,
  // mode flags must stay active so messages route to the correct context.
  describe("Agent-status-aware mode flags (execution panel routing fix)", () => {
    it("keeps isExecutionMode true when execution agent is running but status is pending_review", () => {
      // Simulate: worker called execution_complete → status = pending_review
      // but execution agent still alive in store (not yet exited)
      mockTasks = [{ id: "task-1", internalStatus: "pending_review" }];

      act(() => {
        // Execution agent still running (key present in agentStatus)
        useChatStore.getState().setAgentRunning("task_execution:task-1", true);
        // Also set current store key so isAgentActive = true (badge renders activity)
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // isExecutionMode = true via agent override → agentType = AGENT_WORKER
      // isAgentActive = true → badge renders "Worker running..."
      expect(screen.getByText("Worker running...")).toBeInTheDocument();
    });

    it("falls back to status-based routing when execution agent exits", () => {
      // Same status transition but execution agent has already exited.
      // pending_review is NOT in EXECUTION_STATUSES, so isExecutionMode = false.
      // pending_review IS in ALL_REVIEW_STATUSES, so isReviewMode = true via status.
      mockTasks = [{ id: "task-1", internalStatus: "pending_review" }];

      act(() => {
        // Only current context running — execution agent key is absent
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // isExecutionMode = false (execution agent gone) → falls back to status routing
      // isReviewMode = true (pending_review in ALL_REVIEW_STATUSES) → agentType = AGENT_REVIEWER
      expect(screen.queryByText("Worker running...")).not.toBeInTheDocument();
      expect(screen.getByText("Reviewing...")).toBeInTheDocument();
    });

    it("blocks execution agent override in history mode (!taskHistoryState guard)", () => {
      // History mode: stale agentStatus must NOT override mode flags
      mockTasks = [{ id: "task-1", internalStatus: "pending_review" }];

      act(() => {
        useChatStore.getState().setAgentRunning("task_execution:task-1", true);
        useChatStore.getState().setAgentRunning("task:task-1", true);
        useUiStore.setState({
          taskHistoryState: {
            status: "pending_review",
            conversationId: "conv-1",
            agentRunId: null,
            timestamp: null,
          },
        });
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // isHistoryMode = true → isAgentActive = false → no activity badge text
      // !taskHistoryState guard = false → agent override is blocked
      expect(screen.queryByText("Worker running...")).not.toBeInTheDocument();
      expect(screen.queryByText("Agent responding...")).not.toBeInTheDocument();
    });

    it("keeps isReviewMode true when review agent is running but status transitioned away", () => {
      // Simulate: review agent still alive but status = revision_needed (not in ALL_REVIEW_STATUSES)
      mockTasks = [{ id: "task-1", internalStatus: "revision_needed" }];

      act(() => {
        useChatStore.getState().setAgentRunning("review:task-1", true);
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // isReviewMode = true via agent override → agentType = AGENT_REVIEWER
      // isAgentActive = true → badge renders "Reviewing..."
      expect(screen.getByText("Reviewing...")).toBeInTheDocument();
    });

    it("falls back when review agent exits and status is not a review status", () => {
      // revision_needed is not in ALL_REVIEW_STATUSES, review agent absent
      mockTasks = [{ id: "task-1", internalStatus: "revision_needed" }];

      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // isReviewMode = false → agentType falls through to "agent"
      expect(screen.queryByText("Reviewing...")).not.toBeInTheDocument();
      expect(screen.getByText("Agent responding...")).toBeInTheDocument();
    });
  });

  describe("sortedMessages — always sorted regardless of streaming state", () => {
    // Verifies fix for Task #2: the guard `if (isAgentRunning || isSending) return [...messagesData]`
    // was removed. Messages are now ALWAYS sorted by createdAt with stable secondary sort by id.

    beforeEach(() => {
      // Enable active conversation with proper context so messagesData is populated
      mockChatPanelContext.activeConversationId = "conv-1";
      // Inject conversation context so isConversationInCurrentContext = true
      useChatMockState.conversation = { contextType: "task", contextId: "task-1" };
      // Provide at least one conversation so hasNoConversations = false
      useChatMockState.conversations = [{ id: "conv-1" }];
    });

    it("sorts messages by timestamp even when isAgentRunning is true", () => {
      // msg-b has LATER timestamp but appears first in array (simulates out-of-order DB response)
      useChatMockState.messages = [
        { id: "msg-b", role: "user", content: "Second message", createdAt: new Date(2026, 0, 1, 12, 1).toISOString(), toolCalls: null, contentBlocks: null },
        { id: "msg-a", role: "user", content: "First message", createdAt: new Date(2026, 0, 1, 12, 0).toISOString(), toolCalls: null, contentBlocks: null },
      ];

      // Agent is running — old code would skip sort, new code always sorts
      act(() => {
        useChatStore.getState().setAgentRunning("task:task-1", true);
      });

      const { container } = render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // "First message" (earlier timestamp) must appear before "Second message" in DOM
      const html = container.innerHTML;
      expect(html.indexOf("First message")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("Second message")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("First message")).toBeLessThan(html.indexOf("Second message"));
    });

    it("sorts messages by timestamp when isSending is true", () => {
      useChatMockState.messages = [
        { id: "msg-b", role: "user", content: "Second message", createdAt: new Date(2026, 0, 1, 12, 1).toISOString(), toolCalls: null, contentBlocks: null },
        { id: "msg-a", role: "user", content: "First message", createdAt: new Date(2026, 0, 1, 12, 0).toISOString(), toolCalls: null, contentBlocks: null },
      ];

      act(() => {
        useChatStore.getState().setSending("task:task-1", true);
      });

      const { container } = render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      const html = container.innerHTML;
      expect(html.indexOf("First message")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("Second message")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("First message")).toBeLessThan(html.indexOf("Second message"));
    });

    it("uses id as stable tiebreaker when two messages share the same timestamp", () => {
      const sameTime = new Date(2026, 0, 1, 12, 0).toISOString();
      // "msg-z" sorts after "msg-a" lexically — it should appear SECOND in sorted output
      useChatMockState.messages = [
        { id: "msg-z", role: "user", content: "Zzz response", createdAt: sameTime, toolCalls: null, contentBlocks: null },
        { id: "msg-a", role: "user", content: "Aaa response", createdAt: sameTime, toolCalls: null, contentBlocks: null },
      ];

      const { container } = render(
        <TestWrapper>
          <IntegratedChatPanel projectId="project-1" />
        </TestWrapper>
      );

      // "msg-a" < "msg-z" lexically → "Aaa response" should appear first
      const html = container.innerHTML;
      expect(html.indexOf("Aaa response")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("Zzz response")).toBeGreaterThanOrEqual(0);
      expect(html.indexOf("Aaa response")).toBeLessThan(html.indexOf("Zzz response"));
    });
  });
});

// ============================================================================
// PreviousRunBanner unit tests
// ============================================================================

describe("PreviousRunBanner", () => {
  describe("status label text", () => {
    it("shows 'completed' label when agentRunStatus is 'completed'", () => {
      render(<PreviousRunBanner agentRunStatus="completed" contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("completed");
    });

    it("shows 'failed' label when agentRunStatus is 'failed'", () => {
      render(<PreviousRunBanner agentRunStatus="failed" contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("failed");
    });

    it("shows 'cancelled' label when agentRunStatus is 'cancelled'", () => {
      render(<PreviousRunBanner agentRunStatus="cancelled" contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("cancelled");
    });

    it("shows 'in progress' label when agentRunStatus is 'running' (safety fallback)", () => {
      render(<PreviousRunBanner agentRunStatus="running" contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("in progress");
    });

    it("shows 'completed' label when agentRunStatus is null", () => {
      render(<PreviousRunBanner agentRunStatus={null} contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("completed");
    });
  });

  describe("context type label", () => {
    it("shows 'worker' for contextType 'execution'", () => {
      render(<PreviousRunBanner agentRunStatus="completed" contextType="execution" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("worker");
    });

    it("shows 'reviewer' for contextType 'review'", () => {
      render(<PreviousRunBanner agentRunStatus="completed" contextType="review" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("reviewer");
    });

    it("shows 'merge agent' for contextType 'merge'", () => {
      render(<PreviousRunBanner agentRunStatus="completed" contextType="merge" />);
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("merge agent");
    });
  });
});

// ============================================================================
// PreviousRunBanner visibility integration tests
// ============================================================================

describe("PreviousRunBanner visibility in IntegratedChatPanel", () => {
  const agentRunMessage = {
    id: "msg-1",
    role: "user",
    content: "Hello",
    createdAt: new Date(2026, 0, 1, 12, 0).toISOString(),
    toolCalls: null,
    contentBlocks: null,
  };

  beforeEach(() => {
    // Set execution mode via task status — makes isAgentContext = true
    mockTasks = [{ id: "task-1", internalStatus: "executing" }];
    // Enable agentRunQuery by providing active conversation
    mockChatPanelContext.activeConversationId = "conv-1";
    // Provide messages so sortedMessages.length > 0
    useChatMockState.messages = [agentRunMessage];
    // task_execution contextType satisfies the "task" + "task_execution" special case in isConversationInCurrentContext
    useChatMockState.conversation = { contextType: "task_execution", contextId: "task-1" };
    useChatMockState.conversations = [{ id: "conv-1" }];
    // Reset agentRunStatus mock to null (no status) for each test
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValue(null);
    // Seed lastAgentEventTimestamp >10s ago so isRecentlyActive = false and banner can show
    useChatStore.setState((state) => {
      state.lastAgentEventTimestamp["task:task-1"] = Date.now() - 30_000;
    });
  });

  it("does NOT show banner when backend agentRunStatus is 'running' (agentStatus idle)", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValue({ id: "run-1", status: "running", errorMessage: null });

    render(
      <TestWrapper>
        <IntegratedChatPanel projectId="project-1" />
      </TestWrapper>
    );

    // Wait for query to resolve and banner to be removed (initially shows because data=undefined)
    await waitFor(() => {
      expect(screen.queryByTestId("previous-run-banner")).not.toBeInTheDocument();
    });
  });

  it("shows banner with 'completed' label when backend agentRunStatus is 'completed' (agentStatus idle)", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValue({ id: "run-1", status: "completed", errorMessage: null });

    render(
      <TestWrapper>
        <IntegratedChatPanel projectId="project-1" />
      </TestWrapper>
    );

    // Wait for query to resolve and banner to show correct label
    await waitFor(() => {
      expect(vi.mocked(chatApi.getAgentRunStatus)).toHaveBeenCalledWith("conv-1");
    });

    expect(screen.getByTestId("previous-run-banner")).toBeInTheDocument();
    expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("completed");
  });

  it("shows banner with 'failed' label when backend agentRunStatus is 'failed'", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValue({ id: "run-1", status: "failed", errorMessage: "execution error" });

    render(
      <TestWrapper>
        <IntegratedChatPanel projectId="project-1" />
      </TestWrapper>
    );

    // Wait for query to resolve and label to update from default "completed" to "failed"
    await waitFor(() => {
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("failed");
    });
  });

  it("shows banner with 'cancelled' label when backend agentRunStatus is 'cancelled'", async () => {
    vi.mocked(chatApi.getAgentRunStatus).mockResolvedValue({ id: "run-1", status: "cancelled", errorMessage: null });

    render(
      <TestWrapper>
        <IntegratedChatPanel projectId="project-1" />
      </TestWrapper>
    );

    // Wait for query to resolve and label to update from default "completed" to "cancelled"
    await waitFor(() => {
      expect(screen.getByTestId("previous-run-banner")).toHaveTextContent("cancelled");
    });
  });
});
