/**
 * QuestionInputBanner - Inline question UI above chat input
 *
 * Replaces the standalone AskUserQuestionCard. Renders as a banner
 * above the ChatInput with numbered option chips, slide-in/out
 * animations, and an answered collapsed state.
 *
 * Supports single-select (chip click selects one, dims others) and
 * multi-select (chip click toggles, checkmarks shown) modes.
 */

import { useState, useCallback, useEffect } from "react";
import { Check, X } from "lucide-react";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";

// ============================================================================
// Types
// ============================================================================

export interface QuestionInputBannerProps {
  /** The active question payload (null when showing answered-only state) */
  question: AskUserQuestionPayload | null;
  /** Currently selected option indices (controlled from parent for input sync) */
  selectedIndices: Set<number>;
  /** Called when user clicks a chip */
  onChipClick: (index: number) => void;
  /** Called when user clicks the dismiss (X) button */
  onDismiss: () => void;
  /** When set, shows the collapsed answered state */
  answeredValue?: string | undefined;
  /** Called when user clicks dismiss on the answered banner */
  onDismissAnswered?: (() => void) | undefined;
}

// ============================================================================
// Sub-components
// ============================================================================

function OptionChip({
  index,
  label,
  selected,
  dimmed,
  multiSelect,
  onClick,
}: {
  index: number;
  label: string;
  selected: boolean;
  dimmed: boolean;
  multiSelect: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex items-center gap-1.5 rounded-full text-xs font-medium transition-all"
      style={{
        padding: "5px 12px 5px 8px",
        border: selected
          ? "1px solid rgba(255, 107, 53, 0.4)"
          : "1px solid hsla(220 10% 100% / 0.12)",
        background: selected
          ? "rgba(255, 107, 53, 0.15)"
          : "hsl(220 10% 10%)",
        color: "hsl(220 10% 90%)",
        opacity: dimmed ? 0.45 : 1,
        cursor: "pointer",
        userSelect: "none",
      }}
    >
      {/* Number circle */}
      <span
        className="inline-flex items-center justify-center flex-shrink-0 text-[10px] font-bold"
        style={{
          width: 18,
          height: 18,
          borderRadius: "50%",
          background: selected
            ? "var(--accent-primary)"
            : "hsla(220 10% 100% / 0.06)",
          color: selected ? "#fff" : "hsl(220 10% 45%)",
          transition: "all 0.15s ease",
        }}
      >
        {index + 1}
      </span>

      {/* Checkmark for multi-select */}
      {multiSelect && (
        <span
          className="text-[10px]"
          style={{
            color: selected ? "var(--accent-primary)" : "hsl(220 10% 35%)",
            marginLeft: -2,
          }}
        >
          <Check size={10} strokeWidth={2.5} />
        </span>
      )}

      {/* Label */}
      <span className="font-medium">{label}</span>
    </button>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function QuestionInputBanner({
  question,
  selectedIndices,
  onChipClick,
  onDismiss,
  answeredValue,
  onDismissAnswered,
}: QuestionInputBannerProps) {
  const [visible, setVisible] = useState(false);

  // Trigger slide-in on mount
  useEffect(() => {
    const raf = requestAnimationFrame(() => {
      setVisible(true);
    });
    return () => cancelAnimationFrame(raf);
  }, []);

  const handleDismiss = useCallback(() => {
    setVisible(false);
    // Wait for animation to complete before calling parent dismiss
    setTimeout(onDismiss, 350);
  }, [onDismiss]);

  const handleDismissAnswered = useCallback(() => {
    if (!onDismissAnswered) return;
    setVisible(false);
    setTimeout(onDismissAnswered, 350);
  }, [onDismissAnswered]);

  const isAnswered = answeredValue !== undefined;

  // Nothing to show: no active question and no answered state
  if (!question && !isAnswered) return null;

  return (
    <div
      data-testid="question-input-banner"
      style={{
        overflow: "hidden",
        maxHeight: !visible ? 0 : isAnswered ? 56 : 320,
        opacity: visible ? 1 : 0,
        transition: `max-height 0.35s cubic-bezier(0.22, 1, 0.36, 1),
                     opacity 0.25s ease,
                     padding 0.35s cubic-bezier(0.22, 1, 0.36, 1)`,
        padding: visible ? "12px 12px 0" : "0 12px",
      }}
    >
      <div
        style={{
          background: "hsl(220 10% 12%)",
          border: "1px solid hsla(220 10% 100% / 0.12)",
          borderRadius: 8,
          overflow: "hidden",
        }}
      >
        {isAnswered ? (
          /* ── Answered/collapsed state ── */
          <div
            data-testid="question-input-banner-answered"
            className="flex items-center gap-2"
            style={{ padding: "10px 12px" }}
          >
            {/* Success icon */}
            <span
              className="flex items-center justify-center flex-shrink-0"
              style={{
                width: 18,
                height: 18,
                borderRadius: "50%",
                background: "rgba(52, 211, 153, 0.12)",
                color: "#34d399",
                fontSize: 10,
              }}
            >
              <Check size={10} strokeWidth={3} />
            </span>

            <span className="text-xs" style={{ color: "hsl(220 10% 45%)" }}>
              Answered:
            </span>
            <span
              className="text-xs font-semibold truncate flex-1 min-w-0"
              style={{ color: "hsl(220 10% 90%)" }}
            >
              {answeredValue}
            </span>

            {onDismissAnswered && (
              <button
                type="button"
                onClick={handleDismissAnswered}
                className="flex-shrink-0 flex items-center justify-center transition-colors"
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: 4,
                  border: "none",
                  background: "transparent",
                  color: "hsl(220 10% 35%)",
                  cursor: "pointer",
                }}
                aria-label="Dismiss answered summary"
              >
                <X size={12} />
              </button>
            )}
          </div>
        ) : question ? (
          /* ── Active question state ── */
          <>
            {/* Header row */}
            <div
              className="flex items-center gap-2"
              style={{
                padding: "8px 12px",
                borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
              }}
            >
              {/* ? icon in circle */}
              <span
                className="flex items-center justify-center flex-shrink-0 text-[11px] font-bold"
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: "50%",
                  background: "rgba(255, 107, 53, 0.15)",
                  color: "var(--accent-primary)",
                }}
              >
                ?
              </span>

              <span
                className="text-[11px] font-semibold flex-1"
                style={{ color: "hsl(220 10% 45%)" }}
              >
                {question.header ?? "Question from agent"}
              </span>

              <button
                type="button"
                onClick={handleDismiss}
                className="flex-shrink-0 flex items-center justify-center transition-all"
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: 4,
                  border: "none",
                  background: "transparent",
                  color: "hsl(220 10% 35%)",
                  cursor: "pointer",
                }}
                aria-label="Dismiss question"
              >
                <X size={14} />
              </button>
            </div>

            {/* Body: question text + chips */}
            <div style={{ padding: "10px 12px 12px" }}>
              <p
                className="text-[13px] font-medium leading-snug"
                style={{
                  color: "hsl(220 10% 90%)",
                  marginBottom: 10,
                  lineHeight: 1.45,
                }}
              >
                {question.question}
              </p>

              {/* Option chips */}
              <div className="flex flex-wrap gap-1.5">
                {question.options.map((option, i) => {
                  const isSelected = selectedIndices.has(i);
                  const isDimmed = !question.multiSelect &&
                    selectedIndices.size > 0 &&
                    !isSelected;

                  return (
                    <OptionChip
                      key={option.value ?? option.label}
                      index={i}
                      label={option.label}
                      selected={isSelected}
                      dimmed={isDimmed}
                      multiSelect={question.multiSelect}
                      onClick={() => onChipClick(i)}
                    />
                  );
                })}
              </div>
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}
