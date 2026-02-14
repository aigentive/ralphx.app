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
      expect(textarea).toHaveStyle({ minHeight: "38px" });
    });

    it("has maxHeight style to limit growth", () => {
      render(<ChatInput {...defaultProps} />);
      const textarea = screen.getByTestId("chat-input-textarea");
      expect(textarea).toHaveStyle({ maxHeight: "100px" });
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
      expect(textarea).toHaveStyle({ background: "hsl(220 10% 12%)" });
    });

    it("applies accent color to enabled send button", async () => {
      const user = userEvent.setup();
      render(<ChatInput {...defaultProps} />);
      await user.type(screen.getByTestId("chat-input-textarea"), "Hello");
      const sendButton = screen.getByTestId("chat-input-send");
      expect(sendButton).toHaveStyle({ background: "hsl(14 100% 60%)" });
    });

    it("applies reduced opacity when send button is disabled", async () => {
      render(<ChatInput {...defaultProps} />);
      const sendButton = screen.getByTestId("chat-input-send");
      // Send button should have disabled:opacity-50 class or similar
      expect(sendButton).toHaveClass("disabled:opacity-40");
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
      expect(screen.getByText(/↑ to edit queued/i)).toBeInTheDocument();
    });

    it("shows default hint when no queued messages", () => {
      render(<ChatInput {...defaultProps} hasQueuedMessages={false} />);
      const helperText = screen.getByText(/Enter to send.*Shift\+Enter.*new line/i);
      expect(helperText).toBeInTheDocument();
      expect(helperText.textContent).not.toContain("↑");
    });
  });

  // ============================================================================
  // Agent Stop Button Tests
  // ============================================================================

  describe("agent stop button", () => {
    it("shows Stop button when agent is running and no question mode", () => {
      render(<ChatInput {...defaultProps} isAgentRunning={true} onStop={vi.fn()} />);
      expect(screen.getByTestId("chat-input-stop")).toBeInTheDocument();
    });

    it("does not show Send button when agent is running and no question mode", () => {
      render(<ChatInput {...defaultProps} isAgentRunning={true} onStop={vi.fn()} />);
      expect(screen.queryByTestId("chat-input-send")).not.toBeInTheDocument();
    });

    it("calls onStop when Stop button clicked", async () => {
      const user = userEvent.setup();
      const onStop = vi.fn();
      render(<ChatInput {...defaultProps} isAgentRunning={true} onStop={onStop} />);

      await user.click(screen.getByTestId("chat-input-stop"));

      expect(onStop).toHaveBeenCalled();
    });
  });

  // ============================================================================
  // Three-Branch Button Logic Tests (Agent + Question Mode)
  // ============================================================================

  describe("three-branch button logic (agent + question mode)", () => {
    const questionModeProps = {
      optionCount: 3,
      multiSelect: false,
      onMatchedOptions: vi.fn(),
    };

    it("shows Send button when agent running AND questionMode active", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
        />
      );
      expect(screen.getByTestId("chat-input-send")).toBeInTheDocument();
    });

    it("shows secondary stop button when agent running AND questionMode active", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("chat-input-stop-secondary")).toBeInTheDocument();
    });

    it("does not show primary stop button when agent running AND questionMode active", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("chat-input-stop")).not.toBeInTheDocument();
    });

    it("renders Send and secondary Stop buttons in a flex row", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onStop={vi.fn()}
        />
      );

      const sendButton = screen.getByTestId("chat-input-send");
      const stopButton = screen.getByTestId("chat-input-stop-secondary");

      // Both buttons should exist
      expect(sendButton).toBeInTheDocument();
      expect(stopButton).toBeInTheDocument();

      // Verify they're in the same flex container
      const sendParent = sendButton.parentElement;
      const stopParent = stopButton.parentElement;
      expect(sendParent).toBe(stopParent);
      expect(sendParent).toHaveClass("flex");
    });

    it("secondary stop button calls onStop when clicked", async () => {
      const user = userEvent.setup();
      const onStop = vi.fn();
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onStop={onStop}
        />
      );

      await user.click(screen.getByTestId("chat-input-stop-secondary"));

      expect(onStop).toHaveBeenCalled();
    });

    it("Send button is enabled when input has content", async () => {
      const user = userEvent.setup();
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "answer");

      const sendButton = screen.getByTestId("chat-input-send");
      expect(sendButton).not.toBeDisabled();
    });

    it("Send button is disabled when input is empty", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
        />
      );

      const sendButton = screen.getByTestId("chat-input-send");
      expect(sendButton).toBeDisabled();
    });

    it("secondary stop button has correct styling", () => {
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onStop={vi.fn()}
        />
      );

      const stopButton = screen.getByTestId("chat-input-stop-secondary");
      // Check size
      expect(stopButton).toHaveClass("w-[28px]");
      expect(stopButton).toHaveClass("h-[28px]");
    });

    it("Send button in question mode has correct styling", async () => {
      const user = userEvent.setup();
      render(
        <ChatInput
          {...defaultProps}
          isAgentRunning={true}
          questionMode={questionModeProps}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "answer");

      const sendButton = screen.getByTestId("chat-input-send");
      // Should have the orange accent color when enabled
      expect(sendButton).toHaveStyle({ background: "hsl(14 100% 60%)" });
    });

    it("calls onSend when Send button clicked in question mode", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      render(
        <ChatInput
          onSend={onSend}
          isAgentRunning={true}
          questionMode={questionModeProps}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "answer");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).toHaveBeenCalledWith("answer");
    });

    it("calls onSend (not onQueue) in question mode even with agent running", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      const onQueue = vi.fn();
      render(
        <ChatInput
          onSend={onSend}
          isAgentRunning={true}
          questionMode={questionModeProps}
          onQueue={onQueue}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "answer");
      await user.click(screen.getByTestId("chat-input-send"));

      // Question answers must be delivered immediately — never queue
      expect(onSend).toHaveBeenCalledWith("answer");
      expect(onQueue).not.toHaveBeenCalled();
    });
  });

  // ============================================================================
  // File Attachment Tests
  // ============================================================================

  describe("file attachments", () => {
    it("does not render ChatAttachmentPicker when enableAttachments is false", () => {
      render(<ChatInput {...defaultProps} />);
      expect(screen.queryByTestId("attachment-picker-button")).not.toBeInTheDocument();
    });

    it("renders ChatAttachmentPicker when enableAttachments is true", () => {
      render(<ChatInput {...defaultProps} enableAttachments={true} />);
      expect(screen.getByTestId("attachment-picker-button")).toBeInTheDocument();
    });

    it("does not render ChatAttachmentGallery when attachments array is empty", () => {
      render(<ChatInput {...defaultProps} enableAttachments={true} attachments={[]} />);
      expect(screen.queryByTestId("chat-attachment-gallery")).not.toBeInTheDocument();
    });

    it("does not render ChatAttachmentGallery when attachments is undefined", () => {
      render(<ChatInput {...defaultProps} enableAttachments={true} />);
      expect(screen.queryByTestId("chat-attachment-gallery")).not.toBeInTheDocument();
    });

    it("renders ChatAttachmentGallery when attachments exist", () => {
      const attachments = [
        { id: "1", fileName: "test.txt", fileSize: 1024 },
      ];
      render(<ChatInput {...defaultProps} enableAttachments={true} attachments={attachments} />);
      expect(screen.getByTestId("chat-attachment-gallery")).toBeInTheDocument();
    });

    it("renders ChatAttachmentGallery in compact variant", () => {
      const attachments = [
        { id: "1", fileName: "test.txt", fileSize: 1024 },
      ];
      render(<ChatInput {...defaultProps} enableAttachments={true} attachments={attachments} />);

      // Compact variant uses flex gap-2 overflow-x-auto
      const gallery = screen.getByTestId("chat-attachment-gallery");
      expect(gallery).toHaveClass("flex");
      expect(gallery).toHaveClass("gap-2");
      expect(gallery).toHaveClass("overflow-x-auto");
    });

    it("calls onFilesSelected when files are selected via ChatAttachmentPicker", async () => {
      const user = userEvent.setup();
      const onFilesSelected = vi.fn();
      render(
        <ChatInput
          {...defaultProps}
          enableAttachments={true}
          onFilesSelected={onFilesSelected}
        />
      );

      const file = new File(["content"], "test.txt", { type: "text/plain" });
      const fileInput = screen.getByTestId("attachment-file-input");

      await user.upload(fileInput, file);

      expect(onFilesSelected).toHaveBeenCalledWith([file]);
    });

    it("calls onRemoveAttachment when remove button is clicked", async () => {
      const user = userEvent.setup();
      const onRemoveAttachment = vi.fn();
      const attachments = [
        { id: "1", fileName: "test.txt", fileSize: 1024 },
      ];
      render(
        <ChatInput
          {...defaultProps}
          enableAttachments={true}
          attachments={attachments}
          onRemoveAttachment={onRemoveAttachment}
        />
      );

      const removeButton = screen.getByTestId("remove-attachment");
      await user.click(removeButton);

      expect(onRemoveAttachment).toHaveBeenCalledWith("1");
    });

    it("ChatAttachmentPicker appears to the left of textarea", () => {
      render(<ChatInput {...defaultProps} enableAttachments={true} />);

      const container = screen.getByTestId("chat-input-textarea").parentElement;
      const pickerButton = screen.getByTestId("attachment-picker-button");

      expect(container).toContainElement(pickerButton);

      // Picker should be first child in the flex container
      const children = Array.from(container?.children || []);
      const pickerIndex = children.findIndex(el => el.contains(pickerButton));
      const textareaIndex = children.findIndex(el => el === screen.getByTestId("chat-input-textarea"));

      expect(pickerIndex).toBeLessThan(textareaIndex);
    });

    it("ChatAttachmentGallery appears below textarea and above helper text", () => {
      const attachments = [
        { id: "1", fileName: "test.txt", fileSize: 1024 },
      ];
      render(<ChatInput {...defaultProps} enableAttachments={true} attachments={attachments} />);

      const gallery = screen.getByTestId("chat-attachment-gallery");
      const helperText = screen.getByText(/Enter to send/i);

      // Both should be in the document
      expect(gallery).toBeInTheDocument();
      expect(helperText).toBeInTheDocument();

      // Gallery should be before helper text in DOM order
      // Gallery is wrapped in a div, so we need to find the common parent (chat-input container)
      const galleryWrapper = gallery.parentElement;
      const mainContainer = galleryWrapper?.parentElement;
      const children = Array.from(mainContainer?.children || []);
      const galleryWrapperIndex = children.indexOf(galleryWrapper!);
      const helperIndex = children.indexOf(helperText);

      expect(galleryWrapperIndex).toBeGreaterThan(-1);
      expect(helperIndex).toBeGreaterThan(-1);
      expect(galleryWrapperIndex).toBeLessThan(helperIndex);
    });

    it("maintains existing functionality when attachments are enabled", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);
      const attachments = [
        { id: "1", fileName: "test.txt", fileSize: 1024 },
      ];

      render(
        <ChatInput
          onSend={onSend}
          enableAttachments={true}
          attachments={attachments}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Hello{Enter}");

      expect(onSend).toHaveBeenCalledWith("Hello");
    });

    it("send button still works with attachments enabled", async () => {
      const user = userEvent.setup();
      const onSend = vi.fn().mockResolvedValue(undefined);

      render(
        <ChatInput
          onSend={onSend}
          enableAttachments={true}
        />
      );

      const textarea = screen.getByTestId("chat-input-textarea");
      await user.type(textarea, "Message");
      await user.click(screen.getByTestId("chat-input-send"));

      expect(onSend).toHaveBeenCalledWith("Message");
    });
  });

  // ============================================================================
  // Question Mode Tests
  // ============================================================================

  describe("question mode", () => {
    const questionModeProps = {
      optionCount: 3,
      multiSelect: false,
      onMatchedOptions: vi.fn(),
    };

    beforeEach(() => {
      questionModeProps.onMatchedOptions.mockClear();
    });

    describe("placeholder", () => {
      it("shows question-aware placeholder when questionMode is active", () => {
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );
        expect(
          screen.getByPlaceholderText("Type 1-3 or a custom response...")
        ).toBeInTheDocument();
      });

      it("uses custom placeholder over question mode placeholder when explicitly set", () => {
        render(
          <ChatInput
            {...defaultProps}
            questionMode={questionModeProps}
            placeholder="Custom placeholder"
          />
        );
        // questionMode placeholder takes priority
        expect(
          screen.getByPlaceholderText("Type 1-3 or a custom response...")
        ).toBeInTheDocument();
      });

      it("shows normal placeholder when questionMode is undefined", () => {
        render(<ChatInput {...defaultProps} />);
        expect(
          screen.getByPlaceholderText("Send a message...")
        ).toBeInTheDocument();
      });
    });

    describe("helper text", () => {
      it("shows question-aware helper text when questionMode is active", () => {
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );
        expect(
          screen.getByText(/Enter to send.*Type option number or custom text/i)
        ).toBeInTheDocument();
      });

      it("does not show normal helper text when questionMode is active", () => {
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );
        expect(
          screen.queryByText(/Shift\+Enter for new line/i)
        ).not.toBeInTheDocument();
      });
    });

    describe("single-select number matching", () => {
      it("calls onMatchedOptions with [0] when typing '1'", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "1");

        expect(questionModeProps.onMatchedOptions).toHaveBeenCalledWith([0]);
      });

      it("calls onMatchedOptions with [2] when typing '3'", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "3");

        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([2]);
      });

      it("calls onMatchedOptions([]) for out-of-range number", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "5");

        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });

      it("calls onMatchedOptions([]) for zero", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "0");

        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });

      it("calls onMatchedOptions([]) for non-numeric text", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "hello");

        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });

      it("calls onMatchedOptions([]) when input is cleared", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "1");
        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([0]);

        await user.clear(textarea);
        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });

      it("calls onMatchedOptions([]) for multi-digit numbers in single-select", async () => {
        const user = userEvent.setup();
        render(
          <ChatInput {...defaultProps} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "12");

        // "12" is not a valid single option for 3 options, treat as custom text
        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });
    });

    describe("multi-select number matching", () => {
      const multiProps = {
        optionCount: 5,
        multiSelect: true,
        onMatchedOptions: vi.fn(),
      };

      beforeEach(() => {
        multiProps.onMatchedOptions.mockClear();
      });

      it("matches comma-separated numbers '1,3'", async () => {
        const user = userEvent.setup();
        render(<ChatInput {...defaultProps} questionMode={multiProps} />);

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "1,3");

        expect(multiProps.onMatchedOptions).toHaveBeenLastCalledWith([0, 2]);
      });

      it("matches comma-separated with spaces '1, 3, 5'", async () => {
        const user = userEvent.setup();
        render(<ChatInput {...defaultProps} questionMode={multiProps} />);

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "1, 3, 5");

        expect(multiProps.onMatchedOptions).toHaveBeenLastCalledWith([0, 2, 4]);
      });

      it("filters out-of-range numbers in multi-select", async () => {
        const user = userEvent.setup();
        render(<ChatInput {...defaultProps} questionMode={multiProps} />);

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "1,7");

        // Only 1 is valid (7 is out of range for 5 options)
        expect(multiProps.onMatchedOptions).toHaveBeenLastCalledWith([0]);
      });

      it("calls onMatchedOptions([]) for all-invalid multi-select", async () => {
        const user = userEvent.setup();
        render(<ChatInput {...defaultProps} questionMode={multiProps} />);

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "6,7,8");

        expect(multiProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });

      it("shows multi-select placeholder with correct count", () => {
        render(<ChatInput {...defaultProps} questionMode={multiProps} />);
        expect(
          screen.getByPlaceholderText("Type 1-5 or a custom response...")
        ).toBeInTheDocument();
      });
    });

    describe("does not interfere with send", () => {
      it("still calls onSend when Enter is pressed in question mode", async () => {
        const user = userEvent.setup();
        const onSend = vi.fn().mockResolvedValue(undefined);
        render(
          <ChatInput onSend={onSend} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "2");
        await user.keyboard("{Enter}");

        expect(onSend).toHaveBeenCalledWith("2");
      });

      it("clears matched options after send", async () => {
        const user = userEvent.setup();
        const onSend = vi.fn().mockResolvedValue(undefined);
        render(
          <ChatInput onSend={onSend} questionMode={questionModeProps} />
        );

        const textarea = screen.getByTestId("chat-input-textarea");
        await user.type(textarea, "2");
        await user.keyboard("{Enter}");

        // After send, the last call should clear matched options
        expect(questionModeProps.onMatchedOptions).toHaveBeenLastCalledWith([]);
      });
    });
  });
});
