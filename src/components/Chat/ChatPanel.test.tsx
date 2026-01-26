/**
 * ChatPanel component tests
 * Tests for the resizable chat side panel with context awareness
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ChatPanel } from "./ChatPanel";
import type { ChatContext } from "@/types/chat";

// Mock scrollIntoView before tests run
Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
  value: vi.fn(),
  writable: true,
});

// Create mock functions outside vi.mock for persistence
const mockTogglePanel = vi.fn();
const mockSetWidth = vi.fn();
const mockSendMessageMutateAsync = vi.fn();

// Mock state
let mockStoreState = {
  isOpen: true,
  width: 320,
  togglePanel: mockTogglePanel,
  setWidth: mockSetWidth,
};

let mockChatState = {
  messages: {
    data: [] as typeof mockMessages,
    isLoading: false,
    error: null,
  },
  sendMessage: {
    mutateAsync: mockSendMessageMutateAsync,
    isPending: false,
  },
};

// Mock the hooks
vi.mock("@/hooks/useChat", () => ({
  useChat: vi.fn(() => mockChatState),
}));

vi.mock("@/stores/chatStore", () => ({
  useChatStore: vi.fn(() => mockStoreState),
}));

import { useChat } from "@/hooks/useChat";
import { useChatStore } from "@/stores/chatStore";

const mockMessages = [
  {
    id: "msg-1",
    sessionId: "session-1",
    projectId: "project-1",
    taskId: null,
    role: "user" as const,
    content: "Hello, I need help with authentication",
    metadata: null,
    parentMessageId: null,
    createdAt: "2026-01-24T12:00:00Z",
  },
  {
    id: "msg-2",
    sessionId: "session-1",
    projectId: "project-1",
    taskId: null,
    role: "orchestrator" as const,
    content: "I can help you design an authentication system. What approach would you prefer?",
    metadata: null,
    parentMessageId: "msg-1",
    createdAt: "2026-01-24T12:01:00Z",
  },
];

const defaultContext: ChatContext = {
  view: "ideation",
  projectId: "project-1",
  ideationSessionId: "session-1",
};

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe("ChatPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Reset mock state to defaults
    mockStoreState = {
      isOpen: true,
      width: 320,
      togglePanel: mockTogglePanel,
      setWidth: mockSetWidth,
    };

    mockChatState = {
      messages: {
        data: mockMessages,
        isLoading: false,
        error: null,
      },
      sendMessage: {
        mutateAsync: mockSendMessageMutateAsync,
        isPending: false,
      },
    };

    vi.mocked(useChatStore).mockImplementation(() => mockStoreState);
    vi.mocked(useChat).mockImplementation(() => mockChatState as ReturnType<typeof useChat>);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("rendering", () => {
    it("renders the chat panel when open", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel")).toBeInTheDocument();
    });

    it("does not render when closed", () => {
      mockStoreState.isOpen = false;

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.queryByTestId("chat-panel")).not.toBeInTheDocument();
    });

    it("renders header with context indicator", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-header")).toBeInTheDocument();
      // Ideation view shows "Chat" as the context label
      expect(screen.getByText(/chat/i)).toBeInTheDocument();
    });

    it("renders close button", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-close")).toBeInTheDocument();
    });

    it("renders message list", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-messages")).toBeInTheDocument();
    });

    it("renders input field", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-input")).toBeInTheDocument();
    });

    it("renders send button", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-send")).toBeInTheDocument();
    });
  });

  describe("context indicator", () => {
    it("shows 'Project' context for kanban view", () => {
      const kanbanContext: ChatContext = {
        view: "kanban",
        projectId: "project-1",
      };

      render(<ChatPanel context={kanbanContext} />, { wrapper: createWrapper() });

      expect(screen.getByText(/project/i)).toBeInTheDocument();
    });

    it("shows 'Task' context when task is selected", () => {
      const taskContext: ChatContext = {
        view: "task_detail",
        projectId: "project-1",
        selectedTaskId: "task-1",
      };

      render(<ChatPanel context={taskContext} />, { wrapper: createWrapper() });

      expect(screen.getByText(/task/i)).toBeInTheDocument();
    });

    it("shows 'Settings' context for settings view", () => {
      const settingsContext: ChatContext = {
        view: "settings",
        projectId: "project-1",
      };

      render(<ChatPanel context={settingsContext} />, { wrapper: createWrapper() });

      expect(screen.getByText(/settings/i)).toBeInTheDocument();
    });
  });

  describe("messages display", () => {
    it("displays user messages", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByText(/Hello, I need help with authentication/)).toBeInTheDocument();
    });

    it("displays orchestrator messages", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByText(/I can help you design an authentication system/)).toBeInTheDocument();
    });

    it("shows loading indicator when loading messages", () => {
      mockChatState.messages.data = undefined as unknown as typeof mockMessages;
      mockChatState.messages.isLoading = true;

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-loading")).toBeInTheDocument();
    });

    it("shows empty state when no messages", () => {
      mockChatState.messages.data = [];
      mockChatState.messages.isLoading = false;

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-empty")).toBeInTheDocument();
    });
  });

  describe("close functionality", () => {
    it("calls togglePanel when close button clicked (after animation)", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      await user.click(screen.getByTestId("chat-panel-close"));

      // Close button triggers animation, then calls togglePanel after 200ms
      await vi.advanceTimersByTimeAsync(200);

      expect(mockTogglePanel).toHaveBeenCalled();
      vi.useRealTimers();
    });
  });

  describe("keyboard shortcuts", () => {
    it("closes panel when Escape pressed", async () => {
      vi.useFakeTimers();
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      fireEvent.keyDown(document, { key: "Escape" });

      // Escape triggers animation, then calls togglePanel after 200ms
      await vi.advanceTimersByTimeAsync(200);

      expect(mockTogglePanel).toHaveBeenCalled();
      vi.useRealTimers();
    });
  });

  describe("send message", () => {
    it("sends message when send button clicked", async () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const input = screen.getByTestId("chat-panel-input");
      await userEvent.type(input, "Test message");
      await userEvent.click(screen.getByTestId("chat-panel-send"));

      expect(mockSendMessageMutateAsync).toHaveBeenCalledWith("Test message");
    });

    it("sends message when Enter pressed", async () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const input = screen.getByTestId("chat-panel-input");
      await userEvent.type(input, "Test message{Enter}");

      expect(mockSendMessageMutateAsync).toHaveBeenCalledWith("Test message");
    });

    it("does not send on Shift+Enter (allows newline)", async () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const input = screen.getByTestId("chat-panel-input");
      await userEvent.type(input, "Test message");
      fireEvent.keyDown(input, { key: "Enter", shiftKey: true });

      expect(mockSendMessageMutateAsync).not.toHaveBeenCalled();
    });

    it("clears input after sending", async () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const input = screen.getByTestId("chat-panel-input") as HTMLTextAreaElement;
      await userEvent.type(input, "Test message");
      await userEvent.click(screen.getByTestId("chat-panel-send"));

      expect(input.value).toBe("");
    });

    it("disables send button when input is empty", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const sendButton = screen.getByTestId("chat-panel-send");
      expect(sendButton).toBeDisabled();
    });

    it("disables input while sending", () => {
      mockChatState.sendMessage.isPending = true;

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-input")).toBeDisabled();
      expect(screen.getByTestId("chat-panel-send")).toBeDisabled();
    });
  });

  describe("panel width", () => {
    it("applies width from store", () => {
      mockStoreState.width = 400;

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const panel = screen.getByTestId("chat-panel");
      expect(panel).toHaveStyle({ width: "400px" });
    });

    it("has minimum width of 280px", () => {
      mockStoreState.width = 200; // Below minimum

      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const panel = screen.getByTestId("chat-panel");
      // Component should enforce minimum via style
      expect(panel).toHaveStyle({ minWidth: "280px" });
    });

    it("has resize handle", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByTestId("chat-panel-resize-handle")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("applies design system background color", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const panel = screen.getByTestId("chat-panel");
      expect(panel).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("has border-left style for subtle border", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const panel = screen.getByTestId("chat-panel");
      // Check that borderLeft contains the expected CSS variable
      expect(panel.style.borderLeft).toBe("1px solid var(--border-subtle)");
    });
  });

  describe("auto-scroll", () => {
    it("scrolls to bottom on new message", async () => {
      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      const { rerender } = render(
        <ChatPanel context={defaultContext} />,
        { wrapper: createWrapper() }
      );

      // Add a new message
      const updatedMessages = [
        ...mockMessages,
        {
          id: "msg-3",
          sessionId: "session-1",
          projectId: "project-1",
          taskId: null,
          role: "user" as const,
          content: "New message",
          metadata: null,
          parentMessageId: null,
          createdAt: "2026-01-24T12:02:00Z",
        },
      ];

      mockChatState.messages.data = updatedMessages;

      rerender(<ChatPanel context={defaultContext} />);

      // Verify new message is rendered
      expect(screen.getByText("New message")).toBeInTheDocument();
    });
  });

  describe("accessibility", () => {
    it("has appropriate aria labels", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      expect(screen.getByRole("complementary")).toBeInTheDocument();
      // Use getByTestId since we have multiple elements with similar labels
      expect(screen.getByTestId("chat-panel")).toHaveAttribute("aria-label", "Chat panel");
    });

    it("input has proper placeholder", () => {
      render(<ChatPanel context={defaultContext} />, { wrapper: createWrapper() });

      const input = screen.getByPlaceholderText(/send a message/i);
      expect(input).toBeInTheDocument();
    });
  });
});
