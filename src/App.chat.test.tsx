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
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { act } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { getQueryClient } from "@/lib/queryClient";
import { useChatStore } from "@/stores/chatStore";
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
    messages: { data: [], isLoading: false },
    sendMessage: { mutateAsync: vi.fn(), isPending: false },
  }),
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
    // Reset chat store before each test
    act(() => {
      useChatStore.setState({
        messages: {},
        context: null,
        isOpen: false,
        width: 320,
        isLoading: false,
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
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();
    });

    it("displays header with Chat title", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      expect(screen.getByText("Chat")).toBeInTheDocument();
    });

    it("displays context indicator", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );
      expect(screen.getByText("Kanban")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Keyboard Shortcut
  // ==========================================================================

  describe("keyboard shortcut (Cmd+K)", () => {
    it("opens chat panel on Cmd+K", () => {
      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();

      act(() => {
        fireEvent.keyDown(document, { key: "k", metaKey: true });
      });

      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();
    });

    it("closes chat panel on Cmd+K when open", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();

      act(() => {
        fireEvent.keyDown(document, { key: "k", metaKey: true });
      });

      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();
    });

    it("does not toggle when input is focused", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <div>
            <input data-testid="other-input" />
            <ChatPanel context={mockContext} />
          </div>
        </TestWrapper>
      );

      const input = screen.getByTestId("other-input");
      input.focus();

      act(() => {
        fireEvent.keyDown(document, { key: "k", metaKey: true });
      });

      // Should still be open - shortcut ignored when input focused
      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Close Button
  // ==========================================================================

  describe("close button", () => {
    it("closes chat panel when close button clicked", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      fireEvent.click(screen.getByTestId("chat-panel-close"));

      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Panel Width
  // ==========================================================================

  describe("panel width", () => {
    it("applies width from store", () => {
      act(() => {
        useChatStore.setState({ isOpen: true, width: 400 });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true, width: 280 });
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
    it("togglePanel toggles isOpen state", () => {
      const { togglePanel } = useChatStore.getState();

      expect(useChatStore.getState().isOpen).toBe(false);

      act(() => {
        togglePanel();
      });
      expect(useChatStore.getState().isOpen).toBe(true);

      act(() => {
        togglePanel();
      });
      expect(useChatStore.getState().isOpen).toBe(false);
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
        setWidth(100); // Below MIN_WIDTH of 280
      });

      expect(useChatStore.getState().width).toBe(280);
    });

    it("setWidth clamps to maximum", () => {
      const { setWidth } = useChatStore.getState();

      act(() => {
        setWidth(1000); // Above MAX_WIDTH of 800
      });

      expect(useChatStore.getState().width).toBe(800);
    });

    it("setOpen directly sets isOpen state", () => {
      const { setOpen } = useChatStore.getState();

      act(() => {
        setOpen(true);
      });
      expect(useChatStore.getState().isOpen).toBe(true);

      act(() => {
        setOpen(false);
      });
      expect(useChatStore.getState().isOpen).toBe(false);
    });
  });

  // ==========================================================================
  // Context Awareness
  // ==========================================================================

  describe("context awareness", () => {
    it("shows Kanban context for kanban view", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={{ view: "kanban", projectId: "p1" }} />
        </TestWrapper>
      );

      expect(screen.getByText("Kanban")).toBeInTheDocument();
    });

    it("shows Ideation context for ideation view", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
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

      expect(screen.getByText("Ideation")).toBeInTheDocument();
    });

    it("shows Task context for task_detail view", () => {
      act(() => {
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
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
        useChatStore.setState({ isOpen: true });
      });

      render(
        <TestWrapper>
          <ChatPanel context={mockContext} />
        </TestWrapper>
      );

      const panel = screen.getByTestId("chat-panel");
      // Check border color via style attribute since toHaveStyle has issues with CSS variables
      expect(panel.getAttribute("style")).toContain("border-color: var(--border-subtle)");
    });
  });
});
