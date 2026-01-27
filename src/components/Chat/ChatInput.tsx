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
}: ChatInputProps) {
  // Support both controlled and uncontrolled modes
  const [internalValue, setInternalValue] = useState("");
  const isControlled = controlledValue !== undefined;
  const value = isControlled ? controlledValue : internalValue;

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Determine the actual placeholder text
  const effectivePlaceholder = isAgentRunning
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
      if (e.key === "Enter" && !e.shiftKey) {
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

  // Allow typing and queueing when agent is running
  const isDisabled = isSending && !isAgentRunning;
  const canSend = value.trim().length > 0 && (!isSending || isAgentRunning);

  return (
    <div data-testid="chat-input" className="flex flex-col">
      <div className="flex gap-2 items-end">
        {/* Textarea - Refined Studio styling */}
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
          className="flex-1 px-3 py-2 text-[13px] resize-none rounded-lg outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0 focus:border-0 placeholder:text-white/30"
          style={{
            background: "linear-gradient(180deg, rgba(28,28,28,0.9) 0%, rgba(22,22,22,0.95) 100%)",
            color: "rgba(255,255,255,0.9)",
            border: "1px solid rgba(255,255,255,0.06)",
            minHeight: "38px",
            maxHeight: "100px",
            overflowY: "auto",
            boxShadow: "none",
            outline: "none",
          }}
        />

        {/* Send/Stop Button - Refined Studio gradient */}
        {isAgentRunning && onStop ? (
          <button
            data-testid="chat-input-stop"
            type="button"
            onClick={onStop}
            aria-label="Stop agent"
            className="px-3 py-2 rounded-lg transition-all shrink-0 h-[38px] flex items-center justify-center hover:brightness-110"
            style={{
              background: "linear-gradient(135deg, #ef4444 0%, #dc2626 100%)",
              color: "white",
              boxShadow: "0 2px 8px rgba(239,68,68,0.3)",
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
            className="px-3 py-2 rounded-lg transition-all disabled:opacity-40 shrink-0 h-[38px] flex items-center justify-center hover:brightness-110"
            style={{
              background: canSend
                ? "linear-gradient(135deg, #ff6b35 0%, #e85a28 100%)"
                : "rgba(255,107,53,0.3)",
              color: "white",
              boxShadow: canSend ? "0 2px 8px rgba(255,107,53,0.3)" : "none",
            }}
          >
            {isSending ? <LoadingSpinner /> : <SendIcon />}
          </button>
        )}
      </div>

      {/* Helper Text */}
      {showHelperText && (
        <p className="text-[10px] mt-1.5 text-white/35">
          {hasQueuedMessages
            ? "Enter to send · Shift+Enter for new line · ↑ to edit queued"
            : "Enter to send · Shift+Enter for new line"}
        </p>
      )}
    </div>
  );
}
