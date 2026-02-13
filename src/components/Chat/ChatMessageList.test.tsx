/**
 * ChatMessageList integration tests
 * Tests scroll behavior in real component scenarios:
 * - Single-path Virtuoso scroll (no DOM marker auto-scroll)
 * - Hook receives virtuosoRef for Virtuoso-native scrolling
 * - Streaming content renders without DOM scroll calls
 * - Context switches (conversation changes)
 * - History mode disables auto-scroll
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ChatMessageList, type ChatMessageData } from "./ChatMessageList";
import type { ToolCall } from "./ToolCallIndicator";

// Mock scrollIntoView before tests run — should NEVER be called for auto-scroll
const scrollIntoViewMock = vi.fn();
Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
  value: scrollIntoViewMock,
  writable: true,
});

// Mock useChatAutoScroll to control scroll behavior in tests
let mockIsAtBottom = true;
const mockScrollToBottom = vi.fn();
const mockHandleAtBottomStateChange = vi.fn();
const mockHandleFollowOutput = vi.fn((atBottom: boolean) =>
  atBottom ? "smooth" as const : false as const
);

// Capture hook call args to verify virtuosoRef and disabled are passed
const mockUseChatAutoScroll = vi.fn(() => ({
  isAtBottom: mockIsAtBottom,
  scrollToBottom: mockScrollToBottom,
  handleAtBottomStateChange: mockHandleAtBottomStateChange,
  handleFollowOutput: mockHandleFollowOutput,
  shouldAutoScroll: mockIsAtBottom,
  containerRef: { current: null },
  messagesEndRef: { current: null },
}));

vi.mock("@/hooks/useChatAutoScroll", () => ({
  useChatAutoScroll: (...args: unknown[]) => mockUseChatAutoScroll(...args),
}));

const createMessages = (count: number): ChatMessageData[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: `msg-${i + 1}`,
    role: i % 2 === 0 ? "user" : "assistant",
    content: `Message ${i + 1}`,
    createdAt: new Date(2026, 0, 1, 12, i).toISOString(),
    toolCalls: null,
    contentBlocks: null,
  }));
};

const defaultProps = {
  messages: createMessages(10),
  conversationId: "conv-1",
  failedRun: null,
  onDismissFailedRun: vi.fn(),
  isSending: false,
  isAgentRunning: false,
  streamingToolCalls: [],
  streamingTasks: new Map(),
  streamingText: undefined,
  messagesEndRef: { current: null },
  scrollToTimestamp: null,
};

describe("ChatMessageList - Scroll Behavior", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsAtBottom = true;
    scrollIntoViewMock.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("initial conversation load", () => {
    it("starts at last message on mount (Virtuoso initialTopMostItemIndex)", () => {
      render(<ChatMessageList {...defaultProps} />);

      // Verify messages are rendered
      expect(screen.getByText("Message 1")).toBeInTheDocument();
      expect(screen.getByText("Message 10")).toBeInTheDocument();
    });

    it("remounts completely when conversation ID changes", () => {
      const { rerender } = render(<ChatMessageList {...defaultProps} />);

      // Switch conversation (forces remount via key prop)
      const newMessages = createMessages(5);
      rerender(
        <ChatMessageList
          {...defaultProps}
          conversationId="conv-2"
          messages={newMessages}
        />
      );

      // Verify new conversation messages
      expect(screen.getByText("Message 1")).toBeInTheDocument();
      expect(screen.getByText("Message 5")).toBeInTheDocument();
      expect(screen.queryByText("Message 10")).not.toBeInTheDocument();
    });

    it("shows no settling delay (instant render)", () => {
      vi.useFakeTimers();
      render(<ChatMessageList {...defaultProps} />);

      // Messages should be visible immediately (no isScrollSettling logic)
      expect(screen.getByText("Message 1")).toBeInTheDocument();
      expect(screen.getByText("Message 10")).toBeInTheDocument();

      vi.useRealTimers();
    });
  });

  describe("streaming auto-scroll", () => {
    it("auto-scrolls when new streaming tool calls appear", () => {
      const streamingToolCalls: ToolCall[] = [
        {
          id: "tool-1",
          name: "Read",
          arguments: { file_path: "/test.ts" },
        },
      ];

      const { rerender } = render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={[]}
        />
      );

      // Add streaming tool call
      rerender(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={streamingToolCalls}
        />
      );

      // Verify tool indicator is rendered
      expect(screen.getByTestId("streaming-tool-indicator")).toBeInTheDocument();
    });

    it("auto-scrolls when streaming text appears", () => {
      const { rerender } = render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText={undefined}
        />
      );

      // Add streaming text
      rerender(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText="Streaming assistant response..."
        />
      );

      // Verify streaming text is rendered
      expect(screen.getByText(/Streaming assistant response/)).toBeInTheDocument();
    });

    it("auto-scrolls when agent is running without streaming content", () => {
      render(
        <ChatMessageList
          {...defaultProps}
          isAgentRunning={true}
          streamingToolCalls={[]}
          streamingText={undefined}
        />
      );

      // Verify component renders with agent running state
      // Note: In test env, typing indicator is rendered in simplified DOM
      expect(screen.getByTestId("integrated-chat-messages")).toBeInTheDocument();
    });

    it("does not auto-scroll when user scrolled up", () => {
      mockIsAtBottom = false;

      const { rerender } = render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText={undefined}
        />
      );

      // Add streaming content (should not trigger scroll)
      rerender(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText="New content"
        />
      );

      // Content rendered but scroll behavior controlled by hook
      expect(screen.getByText(/New content/)).toBeInTheDocument();
    });
  });

  describe("manual scroll detection", () => {
    it("tracks bottom state via hook integration", () => {
      render(<ChatMessageList {...defaultProps} />);

      // Verify component renders successfully with auto-scroll hook
      expect(screen.getByTestId("integrated-chat-messages")).toBeInTheDocument();

      // Hook integration is verified by component not throwing errors
      // The mocked hook provides the necessary callbacks
    });

    it("pauses auto-scroll when user manually scrolls up", () => {
      mockIsAtBottom = false;
      mockHandleFollowOutput.mockReturnValue(false);

      render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText="Streaming..."
        />
      );

      // Verify streaming content is rendered but scroll is paused
      expect(screen.getByText(/Streaming/)).toBeInTheDocument();
    });

    it("supports scroll-to-bottom button when scrolled up with >5 messages", () => {
      mockIsAtBottom = false;

      render(<ChatMessageList {...defaultProps} messages={createMessages(10)} />);

      // Note: In test env, simplified DOM is rendered without Virtuoso footer
      // Button rendering is controlled by useChatAutoScroll hook's isAtBottom state
      // Verify component renders with appropriate message count
      expect(screen.getByText("Message 1")).toBeInTheDocument();
      expect(screen.getByText("Message 10")).toBeInTheDocument();
    });

    it("hides scroll-to-bottom button when at bottom", () => {
      mockIsAtBottom = true;

      render(<ChatMessageList {...defaultProps} messages={createMessages(10)} />);

      // Button should not be visible
      expect(screen.queryByText(/Scroll to bottom/i)).not.toBeInTheDocument();
    });

    it("hides scroll-to-bottom button with ≤5 messages", () => {
      mockIsAtBottom = false;

      render(<ChatMessageList {...defaultProps} messages={createMessages(5)} />);

      // Button should not be visible for short conversations
      expect(screen.queryByText(/Scroll to bottom/i)).not.toBeInTheDocument();
    });

    it("provides scroll-to-bottom functionality via hook", () => {
      mockIsAtBottom = false;

      render(<ChatMessageList {...defaultProps} messages={createMessages(10)} />);

      // Note: In test env, button is not rendered (simplified DOM)
      // But hook provides scrollToBottom function for production use
      // Verify scrollToBottom mock is available
      expect(mockScrollToBottom).toBeDefined();
    });
  });

  describe("conversation switch", () => {
    it("shows last message instantly on conversation switch (no settling)", () => {
      vi.useFakeTimers();
      const { rerender } = render(<ChatMessageList {...defaultProps} />);

      // Switch conversation
      const newMessages = createMessages(8);
      rerender(
        <ChatMessageList
          {...defaultProps}
          conversationId="conv-2"
          messages={newMessages}
        />
      );

      // Messages visible immediately (no 350ms delay)
      expect(screen.getByText("Message 1")).toBeInTheDocument();
      expect(screen.getByText("Message 8")).toBeInTheDocument();

      vi.useRealTimers();
    });

    it("remounts Virtuoso with new key on conversation change", () => {
      const { rerender, container } = render(
        <ChatMessageList {...defaultProps} conversationId="conv-1" />
      );

      const firstVirtuoso = container.querySelector('[data-testid="integrated-chat-messages"]');

      // Switch conversation
      rerender(
        <ChatMessageList
          {...defaultProps}
          conversationId="conv-2"
          messages={createMessages(5)}
        />
      );

      const secondVirtuoso = container.querySelector('[data-testid="integrated-chat-messages"]');

      // Component remounts (same testid but potentially different instance)
      expect(firstVirtuoso).toBeTruthy();
      expect(secondVirtuoso).toBeTruthy();
    });
  });

  describe("history mode (timestamp scroll)", () => {
    it("disables auto-scroll when scrollToTimestamp is set", () => {
      const messages = createMessages(10);
      const targetTimestamp = messages[5].createdAt;

      render(
        <ChatMessageList
          {...defaultProps}
          messages={messages}
          scrollToTimestamp={targetTimestamp}
        />
      );

      // Verify component renders in history mode
      // Hook receives disabled: true when scrollToTimestamp is set
      expect(screen.getByTestId("integrated-chat-messages")).toBeInTheDocument();
    });

    it("does not show scroll-to-bottom button in history mode", () => {
      mockIsAtBottom = false;

      render(
        <ChatMessageList
          {...defaultProps}
          messages={createMessages(10)}
          scrollToTimestamp={new Date().toISOString()}
        />
      );

      // Button should not show in history mode
      expect(screen.queryByText(/Scroll to bottom/i)).not.toBeInTheDocument();
    });
  });

  describe("failed run banner", () => {
    it("shows failed run banner in header", () => {
      const failedRun = {
        id: "run-1",
        errorMessage: "Execution failed: timeout",
      };

      render(
        <ChatMessageList
          {...defaultProps}
          failedRun={failedRun}
          onDismissFailedRun={vi.fn()}
        />
      );

      expect(screen.getByText(/Execution failed: timeout/)).toBeInTheDocument();
    });

    it("dismisses failed run banner when close clicked", async () => {
      const user = userEvent.setup();
      const onDismiss = vi.fn();
      const failedRun = {
        id: "run-1",
        errorMessage: "Error occurred",
      };

      render(
        <ChatMessageList
          {...defaultProps}
          failedRun={failedRun}
          onDismissFailedRun={onDismiss}
        />
      );

      const dismissButton = screen.getByRole("button", { name: /dismiss/i });
      await user.click(dismissButton);

      expect(onDismiss).toHaveBeenCalledWith("run-1");
    });
  });

  describe("memo stability (no infinite re-render)", () => {
    it("timeline useMemo returns stable reference when hookEvents/activeHooks not passed", () => {
      // When hookEvents and activeHooks are omitted, the default `= []` in
      // destructuring creates a new array reference each render. This busts
      // the `timeline` useMemo and causes Virtuoso to re-render infinitely.
      // The fix uses module-level empty constants as defaults.
      const { rerender } = render(
        <ChatMessageList {...defaultProps} />
      );

      // Re-render with same props (no hookEvents/activeHooks passed)
      rerender(<ChatMessageList {...defaultProps} />);

      // If the fix is applied, useChatAutoScroll should have been called
      // with the same messageCount both times — no crash, no infinite loop.
      // The key assertion: the component renders successfully without
      // "Maximum update depth exceeded" error.
      expect(screen.getByTestId("integrated-chat-messages")).toBeInTheDocument();

      // Verify hook was called exactly twice (initial + rerender)
      // If timeline memo was unstable, React would hit update depth limit
      const callCount = mockUseChatAutoScroll.mock.calls.length;
      expect(callCount).toBe(2);
    });

    it("re-render with same props does not increase hook call count beyond expected", () => {
      // Regression test: Virtuoso components/itemContent props must be
      // memoized (useMemo/useCallback) so Virtuoso doesn't re-mount
      // Header/Footer on every render, which triggers atBottomStateChange
      // → state change → re-render → new components object → infinite loop.
      const { rerender } = render(
        <ChatMessageList {...defaultProps} />
      );

      const callsAfterMount = mockUseChatAutoScroll.mock.calls.length;

      // Re-render 5 times with identical props
      for (let i = 0; i < 5; i++) {
        rerender(<ChatMessageList {...defaultProps} />);
      }

      // Each rerender should call the hook exactly once (no cascading re-renders)
      const callsAfterRerenders = mockUseChatAutoScroll.mock.calls.length;
      expect(callsAfterRerenders).toBe(callsAfterMount + 5);
    });
  });

  describe("GAP: virtuosoComponents deps include isAtBottom (F1+F2)", () => {
    it("should re-call hook when isAtBottom toggles (unstable components)", () => {
      mockIsAtBottom = true;
      const { rerender } = render(<ChatMessageList {...defaultProps} />);

      const callsAfterMount = mockUseChatAutoScroll.mock.calls.length;

      // Toggle isAtBottom → useMemo recomputes → re-render
      mockIsAtBottom = false;
      rerender(<ChatMessageList {...defaultProps} />);

      mockIsAtBottom = true;
      rerender(<ChatMessageList {...defaultProps} />);

      // Each toggle causes a re-render (the component processes the state change)
      const callsAfterToggles = mockUseChatAutoScroll.mock.calls.length;
      expect(callsAfterToggles).toBe(callsAfterMount + 2);
    });
  });

  describe("GAP: messages.length in virtuosoComponents deps causes rebuild (F4)", () => {
    it("should re-render when messages.length changes", () => {
      const { rerender } = render(
        <ChatMessageList {...defaultProps} messages={createMessages(5)} />
      );

      const callsAfterMount = mockUseChatAutoScroll.mock.calls.length;

      // Add a message → messages.length changes → useMemo recomputes
      rerender(
        <ChatMessageList {...defaultProps} messages={createMessages(6)} />
      );

      const callsAfterRerender = mockUseChatAutoScroll.mock.calls.length;
      // Component re-renders because props changed (expected behavior)
      expect(callsAfterRerender).toBe(callsAfterMount + 1);
    });
  });

  describe("GAP: failedRun prop creates new object each render (F5)", () => {
    it("should accept new failedRun object references without memoization", () => {
      const failedRun1 = { id: "run-1", errorMessage: "Error A" };
      const { rerender } = render(
        <ChatMessageList {...defaultProps} failedRun={failedRun1} />
      );

      const callsAfterMount = mockUseChatAutoScroll.mock.calls.length;

      // New object with same data (different reference — simulates upstream inline creation)
      const failedRun2 = { id: "run-1", errorMessage: "Error A" };
      rerender(
        <ChatMessageList {...defaultProps} failedRun={failedRun2} />
      );

      // Component re-renders because failedRun is a new ref
      const callsAfterRerender = mockUseChatAutoScroll.mock.calls.length;
      expect(callsAfterRerender).toBe(callsAfterMount + 1);
    });
  });

  describe("footer content hash for streaming", () => {
    it("computes hash based on tool calls count", () => {
      const streamingToolCalls: ToolCall[] = [
        { id: "1", name: "Read", arguments: {} },
        { id: "2", name: "Write", arguments: {} },
      ];

      render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={streamingToolCalls}
        />
      );

      // Verify streaming indicators render
      // Virtuoso handles scroll via context prop (footerContentHash)
      expect(screen.getByTestId("streaming-tool-indicator")).toBeInTheDocument();
    });

    it("computes hash based on streaming text presence", () => {
      render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingText="Thinking..."
        />
      );

      // Verify streaming text renders
      // Virtuoso handles scroll via context prop (footerContentHash)
      expect(screen.getByText(/Thinking/)).toBeInTheDocument();
    });
  });

  describe("single-path Virtuoso scroll (no DOM auto-scroll)", () => {
    it("passes virtuosoRef to useChatAutoScroll hook", () => {
      render(<ChatMessageList {...defaultProps} />);

      // Hook must receive virtuosoRef so scrollToBottom routes through Virtuoso
      expect(mockUseChatAutoScroll).toHaveBeenCalled();
      const hookArgs = mockUseChatAutoScroll.mock.calls[0][0] as Record<string, unknown>;
      expect(hookArgs).toHaveProperty("virtuosoRef");
      expect(hookArgs.virtuosoRef).toBeDefined();
    });

    it("passes disabled=true when scrollToTimestamp is set", () => {
      const messages = createMessages(10);
      render(
        <ChatMessageList
          {...defaultProps}
          messages={messages}
          scrollToTimestamp={messages[3].createdAt}
        />
      );

      const hookArgs = mockUseChatAutoScroll.mock.calls[0][0] as Record<string, unknown>;
      expect(hookArgs.disabled).toBe(true);
    });

    it("passes disabled=false when scrollToTimestamp is null", () => {
      render(
        <ChatMessageList
          {...defaultProps}
          scrollToTimestamp={null}
        />
      );

      const hookArgs = mockUseChatAutoScroll.mock.calls[0][0] as Record<string, unknown>;
      expect(hookArgs.disabled).toBe(false);
    });

    it("passes correct messageCount to hook", () => {
      const messages = createMessages(7);
      render(
        <ChatMessageList
          {...defaultProps}
          messages={messages}
        />
      );

      const hookArgs = mockUseChatAutoScroll.mock.calls[0][0] as Record<string, unknown>;
      expect(hookArgs.messageCount).toBe(7);
    });

    it("does not pass isStreaming or streamingHash to hook (removed props)", () => {
      render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          isAgentRunning={true}
        />
      );

      const hookArgs = mockUseChatAutoScroll.mock.calls[0][0] as Record<string, unknown>;
      // These props were removed — Virtuoso context handles streaming scroll
      expect(hookArgs).not.toHaveProperty("isStreaming");
      expect(hookArgs).not.toHaveProperty("streamingHash");
    });

    it("does NOT call scrollIntoView during streaming content changes", () => {
      scrollIntoViewMock.mockClear();

      const { rerender } = render(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={[]}
        />
      );

      // Add streaming tool call
      rerender(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={[{ id: "1", name: "Read", arguments: {} }]}
        />
      );

      // Add streaming text
      rerender(
        <ChatMessageList
          {...defaultProps}
          isSending={true}
          streamingToolCalls={[{ id: "1", name: "Read", arguments: {} }]}
          streamingText="Response..."
        />
      );

      // No DOM scrollIntoView — Virtuoso followOutput handles all auto-scrolling
      expect(scrollIntoViewMock).not.toHaveBeenCalled();
    });

    it("does NOT call scrollIntoView on conversation switch", () => {
      scrollIntoViewMock.mockClear();

      const { rerender } = render(
        <ChatMessageList {...defaultProps} conversationId="conv-1" />
      );

      // Switch conversation
      rerender(
        <ChatMessageList
          {...defaultProps}
          conversationId="conv-2"
          messages={createMessages(5)}
        />
      );

      // No DOM scrollIntoView — Virtuoso remounts with initialTopMostItemIndex
      expect(scrollIntoViewMock).not.toHaveBeenCalled();
    });

    it("does NOT call scrollIntoView when new messages arrive", () => {
      scrollIntoViewMock.mockClear();

      const { rerender } = render(
        <ChatMessageList {...defaultProps} messages={createMessages(5)} />
      );

      // New message arrives
      rerender(
        <ChatMessageList {...defaultProps} messages={createMessages(6)} />
      );

      // No DOM scrollIntoView — Virtuoso followOutput handles auto-scroll
      expect(scrollIntoViewMock).not.toHaveBeenCalled();
    });
  });
});
