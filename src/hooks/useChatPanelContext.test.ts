/**
 * useChatPanelContext hook tests
 *
 * Tests for context switching behavior and conversation selection logic,
 * ensuring no intermediate empty state during context transitions.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useChatPanelContext } from "./useChatPanelContext";
import { useChatStore } from "@/stores/chatStore";

interface MockState {
  activeConversationId: string | null;
  setActiveConversation: ReturnType<typeof vi.fn>;
  clearMessages: ReturnType<typeof vi.fn>;
  setAgentRunning: ReturnType<typeof vi.fn>;
  setSending: ReturnType<typeof vi.fn>;
}

interface ChatContext {
  view: string;
  projectId: string;
  ideationSessionId?: string;
  selectedTaskId?: string;
}

// Mock chat store
vi.mock("@/stores/chatStore", () => ({
  useChatStore: vi.fn(),
  selectActiveConversationId: vi.fn((state: MockState) => state.activeConversationId),
  getContextKey: vi.fn((context: ChatContext) => {
    if (context.view === "ideation") return `ideation:${context.ideationSessionId}`;
    if (context.view === "task_detail") return `task:${context.selectedTaskId}`;
    return `project:${context.projectId}`;
  }),
}));

// Mock chat API
vi.mock("@/api/chat", () => ({
  chatApi: {
    listConversations: vi.fn(),
    getConversation: vi.fn(),
  },
}));

// Mock useChat hook
vi.mock("./useChat", () => ({
  chatKeys: {
    conversation: (id: string) => ["conversation", id],
    conversationList: (type: string, id: string) => ["conversations", type, id],
    agentRun: (id: string) => ["agent-run", id],
  },
}));

interface ConversationData {
  id: string;
  lastMessageAt?: string | null;
  createdAt: string;
}

describe("useChatPanelContext", () => {
  let queryClient: QueryClient;
  let mockStore: MockState;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    // Setup mock store
    mockStore = {
      activeConversationId: null,
      setActiveConversation: vi.fn(),
      clearMessages: vi.fn(),
      setAgentRunning: vi.fn(),
      setSending: vi.fn(),
    };

    (useChatStore as unknown as { mockImplementation: (fn: (selector: ((state: MockState) => unknown) | undefined) => unknown) => void }).mockImplementation((selector) => {
      if (typeof selector === "function") {
        return selector(mockStore);
      }
      return mockStore;
    });

    (useChatStore as unknown as { getState: () => MockState }).getState = vi.fn(() => mockStore);
  });

  afterEach(() => {
    vi.clearAllMocks();
    queryClient.clear();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  describe("context switching", () => {
    it("should clear messages for old context during context change", async () => {
      const { rerender } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: "session-1",
            selectedTaskId: undefined,
            isExecutionMode: false,
            isReviewMode: false,
            isMergeMode: false,
            isHistoryMode: false,
          },
        }
      );

      // Switch to task context
      rerender({
        projectId: "project-1",
        ideationSessionId: undefined,
        selectedTaskId: "task-1",
        isExecutionMode: true,
        isReviewMode: false,
        isMergeMode: false,
        isHistoryMode: false,
      });

      // Verify cleanup was called with correct old context
      await waitFor(() => {
        expect(mockStore.clearMessages).toHaveBeenCalledWith("ideation:session-1");
      });
    });

    it("should NOT set activeConversationId to null during context switch", async () => {
      mockStore.activeConversationId = "conv-1";

      const { rerender } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: "session-1",
            selectedTaskId: undefined,
            isExecutionMode: false,
            isReviewMode: false,
            isMergeMode: false,
            isHistoryMode: false,
          },
        }
      );

      // Verify initial conversation is set
      expect(mockStore.activeConversationId).toBe("conv-1");

      // Switch context
      rerender({
        projectId: "project-1",
        ideationSessionId: undefined,
        selectedTaskId: "task-1",
        isExecutionMode: true,
        isReviewMode: false,
        isMergeMode: false,
        isHistoryMode: false,
      });

      // Verify setActiveConversation(null) was NOT called during context switch
      // (it should only be called by autoSelectConversation if needed)
      const nullCalls = mockStore.setActiveConversation.mock.calls.filter(
        (call: [string | null]) => call[0] === null
      );
      expect(nullCalls.length).toBe(0);
    });
  });

  describe("autoSelectConversation", () => {
    it("should directly select new conversation when current is stale, without intermediate null", async () => {
      mockStore.activeConversationId = "conv-1";

      const { result } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: undefined,
            selectedTaskId: "task-1",
            isExecutionMode: true,
            isReviewMode: false,
            isMergeMode: false,
            isHistoryMode: false,
          },
        }
      );

      const mockConversations: ConversationData[] = [
        {
          id: "conv-2",
          lastMessageAt: "2026-02-11T12:00:00Z",
          createdAt: "2026-02-11T11:00:00Z",
        },
        {
          id: "conv-3",
          lastMessageAt: "2026-02-11T11:30:00Z",
          createdAt: "2026-02-11T11:00:00Z",
        },
      ];

      // Call autoSelectConversation with conversations that don't include conv-1
      act(() => {
        result.current.autoSelectConversation(
          {
            data: mockConversations,
            isLoading: false,
          },
          false, // executionLoading
          false, // reviewLoading
          false  // mergeLoading
        );
      });

      // Should have selected conv-2 (most recent) directly without setting null first
      const calls = mockStore.setActiveConversation.mock.calls;
      expect(calls.length).toBe(1);
      expect(calls[0][0]).toBe("conv-2");

      // Verify no null was set
      const nullCalls = calls.filter((call: [string | null]) => call[0] === null);
      expect(nullCalls.length).toBe(0);
    });

    it("should only set null if new context has no conversations", async () => {
      mockStore.activeConversationId = "conv-1";

      const { result } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: undefined,
            selectedTaskId: "task-1",
            isExecutionMode: true,
            isReviewMode: false,
            isMergeMode: false,
            isHistoryMode: false,
          },
        }
      );

      // Call autoSelectConversation with empty conversation list
      act(() => {
        result.current.autoSelectConversation(
          {
            data: [],
            isLoading: false,
          },
          false, // executionLoading
          false, // reviewLoading
          false  // mergeLoading
        );
      });

      // Should have set null since new context has no conversations
      const calls = mockStore.setActiveConversation.mock.calls;
      expect(calls.length).toBe(1);
      expect(calls[0][0]).toBe(null);
    });

    it("should select most recent conversation by lastMessageAt", async () => {
      mockStore.activeConversationId = "conv-old";

      const { result } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: undefined,
            selectedTaskId: "task-1",
            isExecutionMode: true,
            isReviewMode: false,
            isMergeMode: false,
            isHistoryMode: false,
          },
        }
      );

      const mockConversations: ConversationData[] = [
        {
          id: "conv-1",
          lastMessageAt: "2026-02-11T10:00:00Z",
          createdAt: "2026-02-11T09:00:00Z",
        },
        {
          id: "conv-2",
          lastMessageAt: "2026-02-11T12:00:00Z", // Most recent
          createdAt: "2026-02-11T09:30:00Z",
        },
        {
          id: "conv-3",
          lastMessageAt: "2026-02-11T11:00:00Z",
          createdAt: "2026-02-11T10:00:00Z",
        },
      ];

      act(() => {
        result.current.autoSelectConversation(
          {
            data: mockConversations,
            isLoading: false,
          },
          false,
          false,
          false
        );
      });

      // Should select conv-2 (most recent lastMessageAt)
      expect(mockStore.setActiveConversation).toHaveBeenCalledWith("conv-2");
    });

    it("should not auto-select in history mode with explicit override", async () => {
      const { result } = renderHook(
        (props) => useChatPanelContext(props),
        {
          wrapper,
          initialProps: {
            projectId: "project-1",
            ideationSessionId: undefined,
            selectedTaskId: "task-1",
            isExecutionMode: false,
            isReviewMode: true,
            isMergeMode: false,
            isHistoryMode: true,
            overrideConversationId: "conv-history",
          },
        }
      );

      // Wait for override effect to run
      await waitFor(() => {
        expect(mockStore.setActiveConversation).toHaveBeenCalledWith("conv-history");
      });

      // Clear the mock calls
      mockStore.setActiveConversation.mockClear();

      const mockConversations: ConversationData[] = [
        {
          id: "conv-1",
          lastMessageAt: "2026-02-11T12:00:00Z",
          createdAt: "2026-02-11T11:00:00Z",
        },
      ];

      act(() => {
        result.current.autoSelectConversation(
          {
            data: mockConversations,
            isLoading: false,
          },
          false,
          false,
          false
        );
      });

      // Should not have called setActiveConversation again because we're in history mode
      // with an explicit override
      expect(mockStore.setActiveConversation).not.toHaveBeenCalled();
    });
  });
});
