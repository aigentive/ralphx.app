/**
 * AskUserQuestionCard - Inline chat widget for agent questions
 *
 * Renders in the ChatMessages footer when the agent asks a question.
 * Styled like StreamingToolIndicator with card background.
 * Supports single-select (radio) and multi-select (checkbox) modes,
 * plus an "Other" free-text input option.
 * Collapses to a summary after submission.
 */

import { useState, useCallback } from "react";
import { HelpCircle, Check, Loader2, ChevronDown, ChevronRight } from "lucide-react";
import type {
  AskUserQuestionPayload,
  AskUserQuestionResponse,
} from "@/types/ask-user-question";

// ============================================================================
// Types
// ============================================================================

interface AskUserQuestionCardProps {
  question: AskUserQuestionPayload;
  onSubmit: (response: AskUserQuestionResponse) => void;
  isSubmitting: boolean;
  /** When set, shows collapsed summary instead of interactive form */
  answeredWith?: string | undefined;
}

// ============================================================================
// Sub-components
// ============================================================================

function OptionRadio({
  label,
  description,
  selected,
  onSelect,
}: {
  label: string;
  description?: string | undefined;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="flex items-start gap-2.5 w-full text-left px-2.5 py-2 rounded-md transition-colors"
      style={{
        backgroundColor: selected ? "hsla(14, 100%, 60%, 0.08)" : "transparent",
      }}
    >
      <div
        className="flex-shrink-0 w-3.5 h-3.5 rounded-full border mt-0.5 flex items-center justify-center"
        style={{
          borderColor: selected ? "var(--accent-primary)" : "var(--text-muted)",
          backgroundColor: selected ? "var(--accent-primary)" : "transparent",
        }}
      >
        {selected && (
          <div className="w-1.5 h-1.5 rounded-full bg-white" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <span
          className="text-xs font-medium block"
          style={{ color: selected ? "var(--text-primary)" : "var(--text-secondary)" }}
        >
          {label}
        </span>
        {description && (
          <span
            className="text-[11px] block mt-0.5"
            style={{ color: "var(--text-muted)" }}
          >
            {description}
          </span>
        )}
      </div>
    </button>
  );
}

function OptionCheckbox({
  label,
  description,
  selected,
  onToggle,
}: {
  label: string;
  description?: string | undefined;
  selected: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className="flex items-start gap-2.5 w-full text-left px-2.5 py-2 rounded-md transition-colors"
      style={{
        backgroundColor: selected ? "hsla(14, 100%, 60%, 0.08)" : "transparent",
      }}
    >
      <div
        className="flex-shrink-0 w-3.5 h-3.5 rounded border mt-0.5 flex items-center justify-center"
        style={{
          borderColor: selected ? "var(--accent-primary)" : "var(--text-muted)",
          backgroundColor: selected ? "var(--accent-primary)" : "transparent",
        }}
      >
        {selected && <Check size={10} className="text-white" strokeWidth={3} />}
      </div>
      <div className="flex-1 min-w-0">
        <span
          className="text-xs font-medium block"
          style={{ color: selected ? "var(--text-primary)" : "var(--text-secondary)" }}
        >
          {label}
        </span>
        {description && (
          <span
            className="text-[11px] block mt-0.5"
            style={{ color: "var(--text-muted)" }}
          >
            {description}
          </span>
        )}
      </div>
    </button>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function AskUserQuestionCard({
  question,
  onSubmit,
  isSubmitting,
  answeredWith,
}: AskUserQuestionCardProps) {
  const [selectedOptions, setSelectedOptions] = useState<Set<string>>(new Set());
  const [showOther, setShowOther] = useState(false);
  const [otherText, setOtherText] = useState("");

  const handleRadioSelect = useCallback((value: string) => {
    setSelectedOptions(new Set([value]));
    setShowOther(false);
    setOtherText("");
  }, []);

  const handleCheckboxToggle = useCallback((value: string) => {
    setSelectedOptions((prev) => {
      const next = new Set(prev);
      if (next.has(value)) {
        next.delete(value);
      } else {
        next.add(value);
      }
      return next;
    });
  }, []);

  const handleSelectOther = useCallback(() => {
    if (!question.multiSelect) {
      setSelectedOptions(new Set());
    }
    setShowOther(true);
  }, [question.multiSelect]);

  const handleSubmit = useCallback(() => {
    const response: AskUserQuestionResponse = {
      taskId: question.taskId,
      selectedOptions: Array.from(selectedOptions),
      customResponse: showOther && otherText.trim() ? otherText.trim() : undefined,
    };
    onSubmit(response);
  }, [question.taskId, selectedOptions, showOther, otherText, onSubmit]);

  const hasValidAnswer = selectedOptions.size > 0 || (showOther && otherText.trim().length > 0);

  // Collapsed summary state after submission
  if (answeredWith) {
    return (
      <div
        data-testid="ask-user-question-answered"
        className="rounded-lg overflow-hidden mb-2"
        style={{
          backgroundColor: "var(--bg-elevated)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <div className="flex items-center gap-2 px-3 py-2">
          <Check
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--accent-primary)" }}
          />
          <span
            className="text-xs"
            style={{ color: "var(--text-muted)" }}
          >
            Answered:
          </span>
          <span
            className="text-xs font-medium truncate"
            style={{ color: "var(--text-secondary)" }}
          >
            {answeredWith}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="ask-user-question-card"
      className="rounded-lg overflow-hidden mb-2"
      style={{
        backgroundColor: "var(--bg-elevated)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <HelpCircle
          size={14}
          className="flex-shrink-0"
          style={{ color: "var(--accent-primary)" }}
        />
        <span
          className="text-xs font-medium"
          style={{ color: "var(--text-secondary)" }}
        >
          {question.header}
        </span>
      </div>

      {/* Question text */}
      <div className="px-3 pt-2.5 pb-1.5">
        <p
          className="text-xs leading-relaxed"
          style={{ color: "var(--text-primary)" }}
        >
          {question.question}
        </p>
      </div>

      {/* Options */}
      <div className="px-1.5 pb-1">
        {question.options.map((option) => {
          const optionValue = option.value ?? option.label;
          return question.multiSelect ? (
            <OptionCheckbox
              key={optionValue}
              label={option.label}
              description={option.description}
              selected={selectedOptions.has(optionValue)}
              onToggle={() => handleCheckboxToggle(optionValue)}
            />
          ) : (
            <OptionRadio
              key={optionValue}
              label={option.label}
              description={option.description}
              selected={selectedOptions.has(optionValue)}
              onSelect={() => handleRadioSelect(optionValue)}
            />
          );
        })}

        {/* "Other" option */}
        <button
          type="button"
          onClick={handleSelectOther}
          className="flex items-start gap-2.5 w-full text-left px-2.5 py-2 rounded-md transition-colors"
          style={{
            backgroundColor: showOther ? "hsla(14, 100%, 60%, 0.08)" : "transparent",
          }}
        >
          {question.multiSelect ? (
            <div
              className="flex-shrink-0 w-3.5 h-3.5 rounded border mt-0.5 flex items-center justify-center"
              style={{
                borderColor: showOther ? "var(--accent-primary)" : "var(--text-muted)",
                backgroundColor: showOther ? "var(--accent-primary)" : "transparent",
              }}
            >
              {showOther && <Check size={10} className="text-white" strokeWidth={3} />}
            </div>
          ) : (
            <div
              className="flex-shrink-0 w-3.5 h-3.5 rounded-full border mt-0.5 flex items-center justify-center"
              style={{
                borderColor: showOther ? "var(--accent-primary)" : "var(--text-muted)",
                backgroundColor: showOther ? "var(--accent-primary)" : "transparent",
              }}
            >
              {showOther && <div className="w-1.5 h-1.5 rounded-full bg-white" />}
            </div>
          )}
          <div className="flex-1 min-w-0 flex items-center gap-1">
            <span
              className="text-xs font-medium"
              style={{ color: showOther ? "var(--text-primary)" : "var(--text-secondary)" }}
            >
              Other
            </span>
            {showOther ? (
              <ChevronDown size={12} style={{ color: "var(--text-muted)" }} />
            ) : (
              <ChevronRight size={12} style={{ color: "var(--text-muted)" }} />
            )}
          </div>
        </button>

        {/* Free text input for "Other" */}
        {showOther && (
          <div className="px-2.5 pb-2">
            <textarea
              value={otherText}
              onChange={(e) => setOtherText(e.target.value)}
              placeholder="Type your response..."
              rows={2}
              className="w-full text-xs rounded-md px-2.5 py-2 resize-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-primary)",
                border: "1px solid var(--border-subtle)",
                boxShadow: "none",
                outline: "none",
              }}
            />
          </div>
        )}
      </div>

      {/* Submit button */}
      <div
        className="px-3 py-2 border-t flex justify-end"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <button
          type="button"
          onClick={handleSubmit}
          disabled={!hasValidAnswer || isSubmitting}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-opacity"
          style={{
            backgroundColor: hasValidAnswer ? "var(--accent-primary)" : "var(--text-muted)",
            color: "white",
            opacity: !hasValidAnswer || isSubmitting ? 0.5 : 1,
            cursor: !hasValidAnswer || isSubmitting ? "not-allowed" : "pointer",
          }}
        >
          {isSubmitting ? (
            <>
              <Loader2 size={12} className="animate-spin" />
              Submitting...
            </>
          ) : (
            "Submit"
          )}
        </button>
      </div>
    </div>
  );
}
