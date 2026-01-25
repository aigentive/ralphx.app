/**
 * AskUserQuestionModal - Modal for agent questions requiring user input
 * Renders options as radio buttons (single select) or checkboxes (multi-select)
 * with an always-present "Other" option for custom responses.
 *
 * Uses shadcn/ui Dialog, RadioGroup, Checkbox components.
 */

import { useState, useCallback, useEffect } from "react";
import { Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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

  // Reset state when question changes
  useEffect(() => {
    if (question) {
      setSelectedOptions([]);
      setOtherSelected(false);
      setOtherValue("");
    }
  }, [question]);

  const handleRadioChange = useCallback(
    (value: string) => {
      if (!question || question.multiSelect) return;

      if (value === "__other__") {
        setOtherSelected(true);
        setSelectedOptions([]);
      } else {
        setOtherSelected(false);
        setSelectedOptions([value]);
      }
    },
    [question]
  );

  const handleCheckboxChange = useCallback(
    (label: string, checked: boolean) => {
      if (!question || !question.multiSelect) return;

      setSelectedOptions((prev) =>
        checked ? [...prev, label] : prev.filter((o) => o !== label)
      );
    },
    [question]
  );

  const handleOtherCheckboxChange = useCallback(
    (checked: boolean) => {
      if (!question) return;
      setOtherSelected(checked);
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

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open && !isLoading) {
        onClose();
      }
    },
    [onClose, isLoading]
  );

  if (!question) return null;

  const hasSelection = selectedOptions.length > 0;
  const hasValidOther = otherSelected && otherValue.trim().length > 0;
  const canSubmit = (hasSelection || hasValidOther) && !isLoading;

  // Determine current radio value
  const radioValue = otherSelected ? "__other__" : (selectedOptions[0] || "");

  return (
    <Dialog open={!!question} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="ask-user-question-modal"
        data-task-id={question.taskId}
        data-multi-select={question.multiSelect ? "true" : "false"}
        className="max-w-md"
        hideCloseButton
      >
        <DialogHeader className="flex-col items-start space-y-1.5 pr-0">
          <DialogTitle data-testid="question-header">{question.header}</DialogTitle>
          <DialogDescription data-testid="question-text" className="text-[var(--text-secondary)]">
            {question.question}
          </DialogDescription>
        </DialogHeader>

        <div className="px-6 py-4 space-y-4">
          {question.multiSelect ? (
            // Multi-select: Checkboxes
            <div className="space-y-3">
              {question.options.map((option) => (
                <label
                  key={option.label}
                  className="flex items-start gap-3 cursor-pointer"
                  style={{ opacity: isLoading ? 0.5 : 1 }}
                >
                  <Checkbox
                    checked={selectedOptions.includes(option.label)}
                    disabled={isLoading}
                    onCheckedChange={(checked) =>
                      handleCheckboxChange(option.label, checked === true)
                    }
                    aria-label={option.label}
                    className="mt-0.5 border-[var(--border-subtle)] data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
                  />
                  <div className="flex-1">
                    <span className="text-sm font-medium text-[var(--text-primary)]">
                      {option.label}
                    </span>
                    {option.description && (
                      <p className="text-xs text-[var(--text-muted)]">
                        {option.description}
                      </p>
                    )}
                  </div>
                </label>
              ))}
              {/* Other option for multi-select */}
              <label
                className="flex items-start gap-3 cursor-pointer"
                style={{ opacity: isLoading ? 0.5 : 1 }}
              >
                <Checkbox
                  checked={otherSelected}
                  disabled={isLoading}
                  onCheckedChange={(checked) => handleOtherCheckboxChange(checked === true)}
                  aria-label="Other"
                  className="mt-0.5 border-[var(--border-subtle)] data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
                />
                <span className="text-sm font-medium text-[var(--text-primary)]">Other</span>
              </label>
            </div>
          ) : (
            // Single-select: Radio buttons
            <RadioGroup
              value={radioValue}
              onValueChange={handleRadioChange}
              disabled={isLoading}
              className="space-y-3"
            >
              {question.options.map((option) => (
                <label
                  key={option.label}
                  className="flex items-start gap-3 cursor-pointer"
                  style={{ opacity: isLoading ? 0.5 : 1 }}
                >
                  <RadioGroupItem
                    value={option.label}
                    aria-label={option.label}
                    className="mt-0.5 border-[var(--border-subtle)] text-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
                  />
                  <div className="flex-1">
                    <span className="text-sm font-medium text-[var(--text-primary)]">
                      {option.label}
                    </span>
                    {option.description && (
                      <p className="text-xs text-[var(--text-muted)]">
                        {option.description}
                      </p>
                    )}
                  </div>
                </label>
              ))}
              {/* Other option for single-select */}
              <label
                className="flex items-start gap-3 cursor-pointer"
                style={{ opacity: isLoading ? 0.5 : 1 }}
              >
                <RadioGroupItem
                  value="__other__"
                  aria-label="Other"
                  className="mt-0.5 border-[var(--border-subtle)] text-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
                />
                <span className="text-sm font-medium text-[var(--text-primary)]">Other</span>
              </label>
            </RadioGroup>
          )}

          {/* Other text input */}
          {otherSelected && (
            <div className="ml-7">
              <Label htmlFor="other-input" className="sr-only">
                Other response
              </Label>
              <Input
                id="other-input"
                data-testid="other-input"
                type="text"
                value={otherValue}
                onChange={(e) => setOtherValue(e.target.value)}
                placeholder="Enter your response..."
                disabled={isLoading}
                className="bg-[var(--bg-base)] border-[var(--border-subtle)] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)]"
              />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            onClick={handleSubmit}
            disabled={!canSubmit}
            className="bg-[var(--status-success)] hover:bg-[var(--status-success)]/90 text-white active:scale-[0.98] transition-all"
          >
            {isLoading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            {isLoading ? "Submitting..." : "Submit Answer"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
