/**
 * useQuestionInput — manages chip selection, input value sync, and question-aware send
 *
 * Extracted from ChatPanel to keep it under 500 LOC.
 * Handles selectedOptions state, chip click logic (single/multi-select),
 * onMatchedOptions callback, controlled input value, and question-aware send.
 */

import { useState, useEffect, useCallback } from "react";
import type { AskUserQuestionPayload, AskUserQuestionResponse } from "@/types/ask-user-question";

export interface UseQuestionInputParams {
  activeQuestion: AskUserQuestionPayload | null;
  submitAnswer: (response: AskUserQuestionResponse) => Promise<void>;
  handleSend: (text: string) => Promise<void>;
}

export function useQuestionInput({
  activeQuestion,
  submitAnswer,
  handleSend,
}: UseQuestionInputParams) {
  const [selectedOptions, setSelectedOptions] = useState<Set<number>>(new Set());
  const [questionInputValue, setQuestionInputValue] = useState("");

  // Reset selection when question changes
  useEffect(() => {
    setSelectedOptions(new Set());
    setQuestionInputValue("");
  }, [activeQuestion?.requestId]);

  // Handle chip click → update selection + sync to input
  const handleChipClick = useCallback(
    (index: number) => {
      if (!activeQuestion) return;
      setSelectedOptions((prev: Set<number>) => {
        const next = new Set(prev);
        if (activeQuestion.multiSelect) {
          if (next.has(index)) next.delete(index);
          else next.add(index);
        } else {
          if (next.has(index)) next.clear();
          else { next.clear(); next.add(index); }
        }
        // Sync input value to show selected option labels
        const labels = Array.from(next)
          .sort()
          .map((i) => String(i + 1));
        setQuestionInputValue(labels.join(", "));
        return next;
      });
    },
    [activeQuestion]
  );

  // onMatchedOptions callback — called by ChatInput when user types numbers
  const handleMatchedOptions = useCallback((indices: number[]) => {
    setSelectedOptions(new Set(indices));
  }, []);

  // Question-aware send: if question active, build response and submitAnswer
  const handleQuestionSend = useCallback(
    async (text: string) => {
      if (!activeQuestion) {
        await handleSend(text);
        return;
      }

      const response: AskUserQuestionResponse = {
        requestId: activeQuestion.requestId,
        taskId: activeQuestion.taskId,
        selectedOptions: [],
      };

      if (selectedOptions.size > 0) {
        response.selectedOptions = Array.from(selectedOptions)
          .sort()
          .map((i) => activeQuestion.options[i]?.value ?? activeQuestion.options[i]?.label ?? "");
      } else if (text.trim()) {
        response.customResponse = text.trim();
      } else {
        return; // Nothing to submit
      }

      await submitAnswer(response);
      setSelectedOptions(new Set());
      setQuestionInputValue("");
    },
    [activeQuestion, selectedOptions, submitAnswer, handleSend]
  );

  return {
    selectedOptions,
    questionInputValue,
    setQuestionInputValue,
    handleChipClick,
    handleMatchedOptions,
    handleQuestionSend,
  };
}
