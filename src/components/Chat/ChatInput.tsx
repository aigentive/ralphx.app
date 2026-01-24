/**
 * ChatInput - Reusable chat input component
 *
 * Features:
 * - Textarea with auto-resize (min 40px, max 120px)
 * - Send button with loading state
 * - Enter to send, Shift+Enter for newline
 * - Disabled state while sending
 * - Attach button placeholder for future functionality
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

function AttachIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M13.5 7.5L8 13C6.067 14.933 3.067 14.933 1.5 13C-0.067 11.067 -0.067 8.067 1.5 6.5L7 1C8.381 -0.381 10.619 -0.381 12 1C13.381 2.381 13.381 4.619 12 6L6.5 11.5C5.672 12.328 4.328 12.328 3.5 11.5C2.672 10.672 2.672 9.328 3.5 8.5L9 3"
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
}: ChatInputProps) {
  // Support both controlled and uncontrolled modes
  const [internalValue, setInternalValue] = useState("");
  const isControlled = controlledValue !== undefined;
  const value = isControlled ? controlledValue : internalValue;

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-focus on mount if requested
  useEffect(() => {
    if (autoFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [autoFocus]);

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

  // Handle sending message
  const handleSend = useCallback(async () => {
    const trimmedValue = value.trim();
    if (!trimmedValue || isSending) return;

    try {
      await onSend(trimmedValue);
      // Clear input only on successful send
      if (isControlled) {
        onChangeProp?.("");
      } else {
        setInternalValue("");
      }
    } catch {
      // Don't clear on error - let user retry
    }
  }, [value, isSending, onSend, isControlled, onChangeProp]);

  // Handle keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  const isDisabled = isSending;
  const canSend = value.trim().length > 0 && !isSending;

  return (
    <div data-testid="chat-input" className="flex flex-col">
      <div className="flex gap-2 items-end">
        {/* Attach Button (placeholder for future) */}
        <button
          data-testid="chat-input-attach"
          type="button"
          disabled
          title="Attach files (coming soon)"
          aria-label="Attach file"
          className="p-2 rounded-lg transition-colors disabled:opacity-50"
          style={{
            backgroundColor: "var(--bg-elevated)",
            color: "var(--text-muted)",
          }}
        >
          <AttachIcon />
        </button>

        {/* Textarea */}
        <textarea
          ref={textareaRef}
          data-testid="chat-input-textarea"
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          disabled={isDisabled}
          placeholder={placeholder}
          rows={1}
          aria-label="Message input"
          className="flex-1 px-3 py-2 text-sm resize-none rounded-lg outline-none focus:ring-1 focus:ring-offset-0"
          style={{
            backgroundColor: "var(--bg-elevated)",
            color: "var(--text-primary)",
            minHeight: "40px",
            maxHeight: "120px",
          }}
        />

        {/* Send Button */}
        <button
          data-testid="chat-input-send"
          type="button"
          onClick={handleSend}
          disabled={!canSend}
          aria-label="Send message"
          aria-busy={isSending}
          className="px-3 py-2 rounded-lg transition-colors disabled:opacity-50"
          style={{
            backgroundColor: "var(--accent-primary)",
            color: "var(--text-primary)",
          }}
        >
          {isSending ? <LoadingSpinner /> : <SendIcon />}
        </button>
      </div>

      {/* Helper Text */}
      {showHelperText && (
        <p
          className="text-xs mt-1 ml-12"
          style={{ color: "var(--text-muted)" }}
        >
          Press Enter to send, Shift+Enter for new line
        </p>
      )}
    </div>
  );
}
