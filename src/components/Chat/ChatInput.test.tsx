/**
 * ChatInput Component Tests
 *
 * Tests for the chat input component with:
 * - Auto-resize textarea
 * - Send button
 * - Enter to send, Shift+Enter for newline
 * - Disabled state while sending
 * - Attach button placeholder
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ChatInput } from "./ChatInput";

describe("ChatInput", () => {
  const defaultProps = {
    onSend: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  describe("rendering", () => {
    it("renders the textarea", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input-textarea")).toBeInTheDocument();
    });

    it("renders the send button", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input-send")).toBeInTheDocument();
    });

    it("renders with placeholder text", () => {
      render(<ChatInput {...defaultProps} placeholder="Type a message..." />);
      expect(screen.getByPlaceholderText("Type a message...")).toBeInTheDocument();
    });

    it("renders with default placeholder when not provided", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByPlaceholderText("Send a message...")).toBeInTheDocument();
    });

    it("renders the component container", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Textarea Behavior Tests
  // ============================================================================

  describe("textarea behavior", () => {
    it("updates value when typing", async () => {
      const user = userEvent.setup();
      render(<ChatInput {...defaultProps} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello world");

      expect(textarea).toHaveValue("Hello world");
    });

    it("clears textarea after sending", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.click(screen.getByTestId("chat-input-send"));

      await waitFor(() => {
        expect(textarea).toHaveValue("");
      });
    });

    it("has accessible label", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByLabelText("Message input")).toBeInTheDocument();
    });

    it("accepts controlled value prop", () => {
      render(<ChatInput {...defaultProps} value="Controlled value" />);
      expect(screen.getByTestId("chat-input-textarea")).toHaveValue("Controlled value");
    });

    it("calls onChange when provided", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<ChatInput {...defaultProps} value="" onChange={onChange} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "a");

      expect(onChange).toHaveBeenCalled();
    });
  });

  // ============================================================================
  // Auto-resize Tests
  // ============================================================================

  describe("auto-resize", () => {
    it("has minHeight style for single line", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      expect(textarea).toHaveStyle({ minHeight: "40px" });
    });

    it("has maxHeight style to limit growth", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      expect(textarea).toHaveStyle({ maxHeight: "120px" });
    });

    it("starts with single row", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      expect(textarea).toHaveAttribute("rows", "1");
    });
  });

  // ============================================================================
  // Send Behavior Tests
  // ============================================================================

  describe("send behavior", () => {
    it("calls onSend with trimmed value when send button clicked", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "  Hello world  ");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).toHaveBeenCalledWith("Hello world");
    });

    it("calls onSend when Enter pressed (without Shift)", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.keyboard("{Enter}");

      expect(onSend).toHaveBeenCalledWith("Hello");
    });

    it("does NOT call onSend when Shift+Enter pressed (newline)", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.keyboard("{Shift>}{Enter}{/Shift}");

      expect(onSend).not.toHaveBeenCalled();
    });

    it("does NOT call onSend when textarea is empty", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).not.toHaveBeenCalled();
    });

    it("does NOT call onSend when textarea contains only whitespace", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "   ");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).not.toHaveBeenCalled();
    });
  });

  // ============================================================================
  // Disabled State Tests
  // ============================================================================

  describe("disabled state", () => {
    it("disables textarea when isSending is true", () => {
      render(<ChatInput {...defaultProps} isSending={true} />);
      expect(screen.getByTestId("chat-input-textarea")).toBeDisabled();
    });

    it("disables send button when isSending is true", () => {
      render(<ChatInput {...defaultProps} isSending={true} />);
      expect(screen.getByTestId("chat-input-send")).toBeDisabled();
    });

    it("disables send button when textarea is empty", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input-send")).toBeDisabled();
    });

    it("enables send button when textarea has content", async () => {
      const user = userEvent.setup();
      render(<ChatInput {...defaultProps} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");

      expect(screen.getByTestId("chat-input-send")).not.toBeDisabled();
    });

    it("does NOT call onSend when disabled and Enter pressed", async () => {
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(<ChatInput onSend={onSend} isSending={true} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      // Type before it's disabled (re-render with disabled)
      fireEvent.change(textarea, { target: { value: "Hello" } });
      fireEvent.keyDown(textarea, { key: "Enter", shiftKey: false });

      expect(onSend).not.toHaveBeenCalled();
    });

    it("shows loading indicator on send button when sending", () => {
      render(<ChatInput {...defaultProps} isSending={true} />);
      const sendButton = screen.getByTestId("chat-input-send");
      expect(sendButton).toHaveAttribute("aria-busy", "true");
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("accessibility", () => {
    it("textarea has proper aria-label", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input-textarea")).toHaveAttribute(
        "aria-label",
        "Message input"
      );
    });

    it("send button has proper aria-label", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.getByTestId("chat-input-send")).toHaveAttribute(
        "aria-label",
        "Send message"
      );
    });

    it("renders helper text about keyboard shortcuts", () => {
      render(<ChatInput {...defaultProps} />);
      expect(
        screen.getByText(/Enter to send.*Shift\+Enter.*new line/i)
      ).toBeInTheDocument();
    });

    it("hides helper text when showHelperText is false", () => {
      render(<ChatInput {...defaultProps} showHelperText={false} />);
      expect(
        screen.queryByText(/Enter to send.*Shift\+Enter.*new line/i)
      ).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Focus Behavior Tests
  // ============================================================================

  describe("focus behavior", () => {
    it("can receive focus", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      textarea.focus();
      expect(document.activeElement).toBe(textarea);
    });

    it("autofocuses when autoFocus prop is true", () => {
      render(<ChatInput {...defaultProps} autoFocus={true} />);
      expect(document.activeElement).toBe(screen.getByTestId("chat-input-textarea"));
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("styling", () => {
    it("applies dark surface background to textarea", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      expect(textarea).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("applies accent color to send button", () => {
      render(<ChatInput {...defaultProps} />);
      const sendButton = screen.getByTestId("chat-input-send");
      expect(sendButton).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("applies reduced opacity when send button is disabled", async () => {
      render(<ChatInput {...defaultProps} />);
      const sendButton = screen.getByTestId("chat-input-send");
      // Send button should have disabled:opacity-50 class or similar
      expect(sendButton).toHaveClass("disabled:opacity-50");
    });
  });

  // ============================================================================
  // Error Handling Tests
  // ============================================================================

  describe("error handling", () => {
    it("clears textarea immediately (optimistic UI) even if onSend throws an error", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockRejectedValue(new Error("Send failed"));
      render(<ChatInput onSend={onSend} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.click(screen.getByTestId("chat-input-send"));

      // Textarea is cleared immediately (optimistic UI)
      await waitFor(() => {
        expect(textarea).toHaveValue("");
      });
      expect(onSend).toHaveBeenCalledWith("Hello");
    });
  });

  // ============================================================================
  // Queue Mode Tests
  // ============================================================================

  describe("queue mode", () => {
    it("shows '(will be queued)' placeholder when agent is running", () => {
      render(<ChatInput {...defaultProps} isAgentRunning={true} />);
      expect(screen.getByPlaceholderText("Send a message... (will be queued)")).toBeInTheDocument();
    });

    it("shows normal placeholder when agent is not running", () => {
      render(<ChatInput {...defaultProps} isAgentRunning={false} />);
      expect(screen.getByPlaceholderText("Send a message...")).toBeInTheDocument();
    });

    it("calls onQueue instead of onSend when agent is running", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      const onQueue = vi.fn();
      render(<ChatInput onSend={onSend} onQueue={onQueue} isAgentRunning={true} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onQueue).toHaveBeenCalledWith("Hello");
      expect(onSend).not.toHaveBeenCalled();
    });

    it("clears textarea after queueing message", async () => {
      const user = userEvent.setup();
      const onQueue = vi.fn();
      render(<ChatInput {...defaultProps} onQueue={onQueue} isAgentRunning={true} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.click(screen.getByTestId("chat-input-send"));

      await waitFor(() => {
        expect(textarea).toHaveValue("");
      });
    });

    it("calls onSend when agent is not running (normal flow)", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      const onQueue = vi.fn();
      render(<ChatInput onSend={onSend} onQueue={onQueue} isAgentRunning={false} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).toHaveBeenCalledWith("Hello");
      expect(onQueue).not.toHaveBeenCalled();
    });

    it("queues message on Enter keypress when agent is running", async () => {
      const user = userEvent.setup();
      const onQueue = vi.fn();
      render(<ChatInput {...defaultProps} onQueue={onQueue} isAgentRunning={true} />);

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello{Enter}");

      expect(onQueue).toHaveBeenCalledWith("Hello");
    });
  });

  // ============================================================================
  // Keyboard Navigation Tests
  // ============================================================================

  describe("keyboard navigation", () => {
    it("calls onEditLastQueued when Up arrow pressed in empty input with queued messages", async () => {
      const user = userEvent.setup();
      const onEditLastQueued = vi.fn();
      render(
        <ChatInput
          {...defaultProps}
          hasQueuedMessages={true}
          onEditLastQueued={onEditLastQueued}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      textarea.focus();
      await user.keyboard("{ArrowUp}");

      expect(onEditLastQueued).toHaveBeenCalled();
    });

    it("does NOT call onEditLastQueued when Up arrow pressed with text in input", async () => {
      const user = userEvent.setup();
      const onEditLastQueued = vi.fn();
      render(
        <ChatInput
          {...defaultProps}
          hasQueuedMessages={true}
          onEditLastQueued={onEditLastQueued}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello");
      await user.keyboard("{ArrowUp}");

      expect(onEditLastQueued).not.toHaveBeenCalled();
    });

    it("does NOT call onEditLastQueued when no queued messages", async () => {
      const user = userEvent.setup();
      const onEditLastQueued = vi.fn();
      render(
        <ChatInput
          {...defaultProps}
          hasQueuedMessages={false}
          onEditLastQueued={onEditLastQueued}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      textarea.focus();
      await user.keyboard("{ArrowUp}");

      expect(onEditLastQueued).not.toHaveBeenCalled();
    });

    it("shows hint about Up arrow when queued messages exist", () => {
      render(<ChatInput {...defaultProps} hasQueuedMessages={true} />);
      expect(
        screen.getByText(/↑ to edit last queued message/i)
      ).toBeInTheDocument();
    });

    it("shows default hint when no queued messages", () => {
      render(<ChatInput {...defaultProps} hasQueuedMessages={false} />);
      const helperText = screen.getByText(/Enter to send.*Shift\+Enter.*new line/i);
      expect(helperText).toBeInTheDocument();
      expect(helperText.textContent).not.toContain("↑");
    });
  });
});
