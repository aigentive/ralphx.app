/**
 * ChatInput - Reusable chat input component
 *
 * Design spec: specs/design/pages/chat-panel.md
 * - Refined Studio aesthetic with layered depth
 * - Gradient background on textarea
 * - Orange accent send button
 * - Compact sizing for application UI
 */

import { useState, useRef, useCallback, useEffect } from "react";

// ============================================================================
// Types
// ============================================================================

export interface ChatInputProps {
  /** Callback when message is sent */
  onSend: (message: string) => Promise<void> | void;
  /** Placeholder text for the textarea */
  placeholder?: string;
  /** Whether a message is currently being sent */
  isSending?: boolean;
  /** Controlled value for the textarea */
  value?: string;
  /** Callback when value changes (for controlled mode) */
  onChange?: (value: string) => void;
  /** Show helper text about keyboard shortcuts */
  showHelperText?: boolean;
  /** Auto-focus the textarea on mount */
  autoFocus?: boolean;
  /** Whether an agent is currently running (enables queue mode) */
  isAgentRunning?: boolean;
  /** Callback when message is queued (while agent running) */
  onQueue?: (message: string) => void;
  /** Whether there are queued messages */
  hasQueuedMessages?: boolean;
  /** Callback to edit the last queued message */
  onEditLastQueued?: () => void;
  /** Callback to stop the running agent */
  onStop?: () => void;
  /** Whether the input is in read-only mode (e.g., viewing historical state) */
  isReadOnly?: boolean;
}

// ============================================================================
// Icons
// ============================================================================

function SendIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M14 2L2 7.5L6.5 9.5M14 2L9.5 14L6.5 9.5M14 2L6.5 9.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function LoadingSpinner() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className="animate-spin"
    >
      <circle
        cx="8"
        cy="8"
        r="6"
        stroke="currentColor"
        strokeWidth="2"
        strokeOpacity="0.3"
      />
      <path
        d="M14 8A6 6 0 0 0 8 2"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
      <rect x="3" y="3" width="10" height="10" rx="1" />
    </svg>
  );
}

// ============================================================================
// Component
// ============================================================================

export function ChatInput({
  onSend,
  placeholder = "Send a message...",
  isSending = false,
  value: controlledValue,
  onChange: onChangeProp,
  showHelperText = true,
  autoFocus = false,
  isAgentRunning = false,
  onQueue,
  hasQueuedMessages = false,
  onEditLastQueued,
  onStop,
  isReadOnly = false,
}: ChatInputProps) {
  // Support both controlled and uncontrolled modes
  const [internalValue, setInternalValue] = useState("");
  const isControlled = controlledValue !== undefined;
  const value = isControlled ? controlledValue : internalValue;

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Determine the actual placeholder text
  const effectivePlaceholder = isReadOnly
    ? "Viewing historical state (read-only)"
    : isAgentRunning
      ? `${placeholder} (will be queued)`
      : placeholder;

  // Auto-focus on mount if requested
  useEffect(() => {
    if (autoFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [autoFocus]);

  // Auto-resize textarea based on content
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    // Reset height to auto to get the correct scrollHeight
    textarea.style.height = "auto";
    // Set height to scrollHeight, capped at maxHeight (120px)
    const newHeight = Math.min(textarea.scrollHeight, 120);
    textarea.style.height = `${newHeight}px`;
  }, [value]);

  // Handle value changes
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      const newValue = e.target.value;
      if (isControlled) {
        onChangeProp?.(newValue);
      } else {
        setInternalValue(newValue);
      }
    },
    [isControlled, onChangeProp]
  );

  // Handle sending or queueing message
  const handleSend = useCallback(async () => {
    const trimmedValue = value.trim();
    // Block if no content, or if sending and agent not running (can't queue)
    if (!trimmedValue || (isSending && !isAgentRunning)) return;

    // Clear input immediately (optimistic UI)
    const clearInput = () => {
      if (isControlled) {
        onChangeProp?.("");
      } else {
        setInternalValue("");
      }
    };

    // If agent is running, queue the message instead of sending
    if (isAgentRunning && onQueue) {
      onQueue(trimmedValue);
      clearInput();
    } else {
      // Normal send flow - clear immediately, don't wait for response
      clearInput();
      try {
        await onSend(trimmedValue);
      } catch {
        // Message was already cleared - error will be shown elsewhere
      }
    }
  }, [
    value,
    isSending,
    isAgentRunning,
    onQueue,
    onSend,
    isControlled,
    onChangeProp,
  ]);

  // Handle keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Escape") {
        e.preventDefault();
        (e.target as HTMLTextAreaElement).blur();
      } else if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      } else if (e.key === "ArrowUp" && !value && hasQueuedMessages) {
        // Up arrow in empty input: edit last queued message
        e.preventDefault();
        onEditLastQueued?.();
      }
    },
    [handleSend, value, hasQueuedMessages, onEditLastQueued]
  );

  // Allow typing and queueing when agent is running, but not in read-only mode
  const isDisabled = isReadOnly || (isSending && !isAgentRunning);
  const canSend = value.trim().length > 0 && !isReadOnly && (!isSending || isAgentRunning);

  return (
    <div data-testid="chat-input" className="flex flex-col">
      <div className="flex gap-2 items-end">
        {/* Textarea - macOS Tahoe flat styling */}
        <textarea
          ref={textareaRef}
          data-testid="chat-input-textarea"
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          disabled={isDisabled}
          placeholder={effectivePlaceholder}
          rows={1}
          aria-label="Message input"
          className="flex-1 px-3 py-2 text-[13px] resize-none rounded-lg outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0 focus:border-0"
          style={{
            /* macOS Tahoe: flat solid color, no gradient */
            background: "hsl(220 10% 12%)",
            color: "hsl(220 10% 90%)",
            border: "none",
            minHeight: "38px",
            maxHeight: "100px",
            overflowY: "auto",
            boxShadow: "none",
            outline: "none",
          }}
        />

        {/* Send/Stop Button - macOS Tahoe flat styling */}
        {/* Only show stop button if agent is running AND not in read-only mode */}
        {isAgentRunning && onStop && !isReadOnly ? (
          <button
            data-testid="chat-input-stop"
            type="button"
            onClick={onStop}
            aria-label="Stop agent"
            className="px-3 py-2 rounded-lg transition-colors shrink-0 h-[38px] flex items-center justify-center hover:brightness-110"
            style={{
              /* macOS Tahoe: flat solid color */
              background: "hsl(0 70% 55%)",
              color: "white",
              boxShadow: "none",
            }}
          >
            <StopIcon />
          </button>
        ) : (
          <button
            data-testid="chat-input-send"
            type="button"
            onClick={handleSend}
            disabled={!canSend}
            aria-label="Send message"
            aria-busy={isSending}
            className="px-3 py-2 rounded-lg transition-colors disabled:opacity-40 shrink-0 h-[38px] flex items-center justify-center hover:brightness-110"
            style={{
              /* macOS Tahoe: flat solid color */
              background: canSend
                ? "hsl(14 100% 60%)"
                : "hsla(14 100% 60% / 0.3)",
              color: "white",
              boxShadow: "none",
            }}
          >
            {isSending ? <LoadingSpinner /> : <SendIcon />}
          </button>
        )}
      </div>

      {/* Helper Text - macOS Tahoe muted styling */}
      {showHelperText && (
        <p
          className="text-[10px] mt-1.5"
          style={{ color: "hsl(220 10% 45%)" }}
        >
          {hasQueuedMessages
            ? "Enter to send · Shift+Enter for new line · ↑ to edit queued"
            : "Enter to send · Shift+Enter for new line"}
        </p>
      )}
    </div>
  );
}
