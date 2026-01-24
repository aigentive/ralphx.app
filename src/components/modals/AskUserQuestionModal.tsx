/**
 * AskUserQuestionModal - Modal for agent questions requiring user input
 * Renders options as radio buttons (single select) or checkboxes (multi-select)
 * with an always-present "Other" option for custom responses.
 */

import { useState, useCallback } from "react";
import type {
  AskUserQuestionPayload,
  AskUserQuestionResponse,
} from "@/types/ask-user-question";

interface AskUserQuestionModalProps {
  question: AskUserQuestionPayload | null;
  onSubmit: (response: AskUserQuestionResponse) => void;
  onClose: () => void;
  isLoading: boolean;
}

export function AskUserQuestionModal({
  question,
  onSubmit,
  onClose,
  isLoading,
}: AskUserQuestionModalProps) {
  const [selectedOptions, setSelectedOptions] = useState<string[]>([]);
  const [otherSelected, setOtherSelected] = useState(false);
  const [otherValue, setOtherValue] = useState("");

  const handleOptionChange = useCallback(
    (label: string, checked: boolean) => {
      if (!question) return;

      if (question.multiSelect) {
        setSelectedOptions((prev) =>
          checked ? [...prev, label] : prev.filter((o) => o !== label)
        );
      } else {
        setSelectedOptions(checked ? [label] : []);
        if (checked) setOtherSelected(false);
      }
    },
    [question]
  );

  const handleOtherChange = useCallback(
    (checked: boolean) => {
      if (!question) return;

      setOtherSelected(checked);
      if (!question.multiSelect && checked) {
        setSelectedOptions([]);
      }
    },
    [question]
  );

  const handleSubmit = useCallback(() => {
    if (!question) return;

    const response: AskUserQuestionResponse = {
      taskId: question.taskId,
      selectedOptions: otherSelected ? [] : selectedOptions,
    };

    if (otherSelected && otherValue.trim()) {
      response.customResponse = otherValue.trim();
    }

    onSubmit(response);
    setSelectedOptions([]);
    setOtherSelected(false);
    setOtherValue("");
  }, [question, selectedOptions, otherSelected, otherValue, onSubmit]);

  const handleOverlayClick = useCallback(() => {
    onClose();
  }, [onClose]);

  if (!question) return null;

  const hasSelection = selectedOptions.length > 0;
  const hasValidOther = otherSelected && otherValue.trim().length > 0;
  const canSubmit = (hasSelection || hasValidOther) && !isLoading;

  const inputType = question.multiSelect ? "checkbox" : "radio";
  const btnBase = "px-4 py-2 rounded text-sm font-medium transition-colors";

  return (
    <div
      data-testid="ask-user-question-modal"
      data-task-id={question.taskId}
      data-multi-select={question.multiSelect ? "true" : "false"}
      className="fixed inset-0 z-50 flex items-center justify-center"
    >
      <div
        data-testid="modal-overlay"
        className="absolute inset-0"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={handleOverlayClick}
      />
      <div
        data-testid="modal-content"
        className="relative w-full max-w-md p-6 rounded-lg shadow-lg"
        style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <h2
          data-testid="question-header"
          className="text-lg font-semibold mb-2"
          style={{ color: "var(--text-primary)" }}
        >
          {question.header}
        </h2>
        <p
          data-testid="question-text"
          className="text-sm mb-4"
          style={{ color: "var(--text-secondary)" }}
        >
          {question.question}
        </p>

        <div className="space-y-3 mb-6">
          {question.options.map((option) => (
            <label
              key={option.label}
              className="flex items-start gap-3 cursor-pointer"
              style={{ opacity: isLoading ? 0.5 : 1 }}
            >
              <input
                type={inputType}
                name="question-option"
                checked={selectedOptions.includes(option.label)}
                disabled={isLoading}
                onChange={(e) => handleOptionChange(option.label, e.target.checked)}
                className="mt-1"
                aria-label={option.label}
              />
              <div>
                <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                  {option.label}
                </span>
                {option.description && (
                  <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {option.description}
                  </p>
                )}
              </div>
            </label>
          ))}

          <label
            className="flex items-start gap-3 cursor-pointer"
            style={{ opacity: isLoading ? 0.5 : 1 }}
          >
            <input
              type={inputType}
              name="question-option"
              checked={otherSelected}
              disabled={isLoading}
              onChange={(e) => handleOtherChange(e.target.checked)}
              className="mt-1"
              aria-label="Other"
            />
            <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
              Other
            </span>
          </label>

          {otherSelected && (
            <input
              data-testid="other-input"
              type="text"
              value={otherValue}
              onChange={(e) => setOtherValue(e.target.value)}
              placeholder="Enter your response..."
              disabled={isLoading}
              className="w-full px-3 py-2 rounded border text-sm ml-6"
              style={{
                backgroundColor: "var(--bg-base)",
                borderColor: "var(--border-subtle)",
                color: "var(--text-primary)",
              }}
            />
          )}
        </div>

        <div className="flex justify-end">
          <button
            onClick={handleSubmit}
            disabled={!canSubmit}
            className={btnBase}
            style={{
              backgroundColor: canSubmit ? "var(--status-success)" : "var(--bg-hover)",
              color: canSubmit ? "var(--bg-base)" : "var(--text-secondary)",
              cursor: canSubmit ? "pointer" : "not-allowed",
            }}
          >
            {isLoading ? "Submitting..." : "Submit Answer"}
          </button>
        </div>
      </div>
    </div>
  );
}
