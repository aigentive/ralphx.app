/**
 * App chat panel integration tests
 *
 * Tests for:
 * - Chat panel integration with App layout
 * - Cmd+K keyboard shortcut toggle
 * - Chat store state management
 * - Panel width persistence in localStorage
 * - Chat opens/closes correctly across views
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { act } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { getQueryClient } from "@/lib/queryClient";
import { useChatStore } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { ChatPanel } from "@/components/Chat/ChatPanel";
import type { ChatContext } from "@/types/chat";

// Mock Tauri API calls
vi.mock("@/lib/tauri", () => ({
  api: {
    execution: {
      pause: vi.fn(),
      resume: vi.fn(),
      stop: vi.fn(),
    },
    tasks: {
      list: vi.fn().mockResolvedValue([]),
    },
    reviews: {
      listPending: vi.fn().mockResolvedValue([]),
    },
    ideation: {
      listMessages: vi.fn().mockResolvedValue([]),
      sendMessage: vi.fn().mockResolvedValue({ id: "msg-1", content: "response" }),
    },
  },
}));

// Mock useChat hook
vi.mock("@/hooks/useChat", () => ({
  useChat: () => ({
    messages: { data: { messages: [] }, isLoading: false },
    sendMessage: { mutateAsync: vi.fn(), isPending: false },
    conversations: { data: [], isLoading: false },
    switchConversation: vi.fn(),
    createConversation: vi.fn(),
  }),
  chatKeys: {
    all: ["chat"],
    messages: () => ["chat", "messages"],
    conversations: () => ["chat", "conversations"],
    conversation: (id: string) => ["chat", "conversations", id],
    conversationList: (type: string, id: string) => ["chat", "conversations", type, id],
    agentRun: (id: string) => ["chat", "agent-run", id],
  },
}));

// ============================================================================
// Test Data
// ============================================================================

const mockContext: ChatContext = {
  view: "kanban",
  projectId: "demo-project",
};

// ============================================================================
// Test Wrapper
// ============================================================================

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = getQueryClient();
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("ChatPanel Integration", () => {
  beforeEach(() => {
    // Reset chat store before each test (no isOpen - visibility is in uiStore now)
    act(() => {
      useChatStore.setState({
        messages: {},
        context: null,
        width: 320,
        isLoading: false,
      });
    });
    // Reset UI store chat visibility
    act(() => {
      useUiStore.setState({
        chatVisibleByView: {
          kanban: false,
          ideation: false,
          extensibility: false,
          activity: false,
          settings: false,
          task_detail: false,
        },
      });
    });
    // Clear localStorage
    localStorage.clear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("does not render ChatPanel when closed", () => {
      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();
    });

    it("renders ChatPanel when open", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();
    });

    it("displays context indicator for kanban view", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      // Kanban view without selected task shows "Project" as context label
      expect(screen.getByText("Project")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Keyboard Shortcut
  // ==========================================================================
  // NOTE: Cmd+K keyboard shortcut is handled by App.tsx, not ChatPanel.
  // Tests for Cmd+K should be in App.test.tsx which renders the full App.
  // ==========================================================================

  // ==========================================================================
  // Close Button
  // ==========================================================================

  describe("close button", () => {
    it("closes chat panel when close button clicked", async () => {
      vi.useFakeTimers();
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      fireEvent.click(screen.getByTestId("chat-panel-close"));

      // Wait for animation to complete (200ms)
      await vi.advanceTimersByTimeAsync(200);

      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();
      vi.useRealTimers();
    });
  });

  // ==========================================================================
  // Panel Width
  // ==========================================================================

  describe("panel width", () => {
    it("applies width from store", () => {
      act(() => {
        useChatStore.setState({ width: 400 });
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      const panel = screen.getByTestId("chat-panel");
      expect(panel).toHaveStyle({ width: "400px" });
    });

    it("has resize handle", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByTestId("chat-panel-resize-handle")).toBeInTheDocument();
    });

    it("uses minimum width from store constants", () => {
      act(() => {
        useChatStore.setState({ width: 280 });
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      const panel = screen.getByTestId("chat-panel");
      expect(panel).toHaveStyle({ minWidth: "280px" });
    });
  });

  // ==========================================================================
  // Store State Management
  // ==========================================================================

  describe("store state management", () => {
    it("toggleChatVisible toggles visibility for a view", () => {
      const { toggleChatVisible } = useUiStore.getState();

      expect(useUiStore.getState().chatVisibleByView.kanban).toBe(false);

      act(() => {
        toggleChatVisible("kanban");
      });
      expect(useUiStore.getState().chatVisibleByView.kanban).toBe(true);

      act(() => {
        toggleChatVisible("kanban");
      });
      expect(useUiStore.getState().chatVisibleByView.kanban).toBe(false);
    });

    it("setWidth updates width in store", () => {
      const { setWidth } = useChatStore.getState();

      act(() => {
        setWidth(450);
      });

      expect(useChatStore.getState().width).toBe(450);
    });

    it("setWidth clamps to minimum", () => {
      const { setWidth } = useChatStore.getState();

      act(() => {
        setWidth(100); // Below MIN_WIDTH of 320
      });

      expect(useChatStore.getState().width).toBe(320);
    });

    it("setWidth clamps to maximum", () => {
      const { setWidth } = useChatStore.getState();

      act(() => {
        setWidth(1000); // Above MAX_WIDTH of 800
      });

      expect(useChatStore.getState().width).toBe(800);
    });

    it("setChatVisible directly sets visibility state for a view", () => {
      const { setChatVisible } = useUiStore.getState();

      act(() => {
        setChatVisible("kanban", true);
      });
      expect(useUiStore.getState().chatVisibleByView.kanban).toBe(true);

      act(() => {
        setChatVisible("kanban", false);
      });
      expect(useUiStore.getState().chatVisibleByView.kanban).toBe(false);
    });
  });

  // ==========================================================================
  // Context Awareness
  // ==========================================================================

  describe("context awareness", () => {
    it("shows Project context for kanban view without selected task", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={{ view: "kanban", projectId: "p1" }} />
        </TestWrapper>
      );

      // Kanban view without selected task shows "Project" as context label
      expect(screen.getByText("Project")).toBeInTheDocument();
    });

    it("shows Chat context for ideation view", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, ideation: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel
            context={{
              view: "ideation",
              projectId: "p1",
              ideationSessionId: "s1",
            }}
          />
        </TestWrapper>
      );

      // Ideation view shows "Chat" as the context label
      expect(screen.getByText("Chat")).toBeInTheDocument();
    });

    it("shows Task context for task_detail view", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, task_detail: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel
            context={{
              view: "task_detail",
              projectId: "p1",
              selectedTaskId: "t1",
            }}
          />
        </TestWrapper>
      );

      expect(screen.getByText("Task")).toBeInTheDocument();
    });

    it("shows Task context for kanban with selected task", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel
            context={{
              view: "kanban",
              projectId: "p1",
              selectedTaskId: "t1",
            }}
          />
        </TestWrapper>
      );

      expect(screen.getByText("Task")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has complementary role", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByRole("complementary")).toBeInTheDocument();
    });

    it("has accessible label", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByLabelText("Chat panel")).toBeInTheDocument();
    });

    it("close button has accessible label", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByLabelText("Close chat panel")).toBeInTheDocument();
    });

    it("input has accessible label", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByLabelText("Message input")).toBeInTheDocument();
    });

    it("send button has accessible label", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByLabelText("Send message")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      const panel = screen.getByTestId("chat-panel");
      expect(panel).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses design tokens for border", () => {
      act(() => {
        useUiStore.setState({
          chatVisibleByView: { ...useUiStore.getState().chatVisibleByView, kanban: true },
        });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      const panel = screen.getByTestId("chat-panel");
      // Check border-left style which includes the design token
      expect(panel.getAttribute("style")).toContain("border-left: 1px solid var(--border-subtle)");
    });
  });
});
