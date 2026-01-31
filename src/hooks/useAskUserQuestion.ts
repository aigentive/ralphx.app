/**
 * useAskUserQuestion hook - Handle agent questions requiring user input
 *
 * Listens for agent:ask_user_question Tauri events, stores the question
 * payload in uiStore, and provides functions to submit answers back to
 * the agent.
 */

import { useEffect, useState, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/tauri";
import { useUiStore } from "@/stores/uiStore";
import {
  AskUserQuestionPayloadSchema,
  type AskUserQuestionResponse,
} from "@/types/ask-user-question";

/**
 * Hook to handle agent questions requiring user input
 *
 * Listens to 'agent:ask_user_question' events and manages the question
 * lifecycle including display and answer submission.
 *
 * @returns Object with activeQuestion, submitAnswer, clearQuestion, and isLoading
 *
 * @example
 * ```tsx
 * function AskUserQuestionModal() {
 *   const { activeQuestion, submitAnswer, clearQuestion, isLoading } = useAskUserQuestion();
 *
 *   if (!activeQuestion) return null;
 *
 *   return (
 *     <Modal onClose={clearQuestion}>
 *       <h2>{activeQuestion.header}</h2>
 *       <p>{activeQuestion.question}</p>
 *       <Options
 *         options={activeQuestion.options}
 *         multiSelect={activeQuestion.multiSelect}
 *         onSubmit={(selected) => submitAnswer({
 *           taskId: activeQuestion.taskId,
 *           selectedOptions: selected,
 *         })}
 *       />
 *     </Modal>
 *   );
 * }
 * ```
 */
export function useAskUserQuestion() {
  const [isLoading, setIsLoading] = useState(false);
  const activeQuestion = useUiStore((s) => s.activeQuestion);
  const setActiveQuestion = useUiStore((s) => s.setActiveQuestion);
  const clearActiveQuestion = useUiStore((s) => s.clearActiveQuestion);

  // Set up event listener for agent questions
  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("agent:ask_user_question", (event) => {
      // Runtime validation of event payload
      const parsed = AskUserQuestionPayloadSchema.safeParse(event.payload);

      if (!parsed.success) {
        return;
      }

      setActiveQuestion(parsed.data);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setActiveQuestion]);

  /**
   * Submit an answer to the agent's question
   * Calls the Tauri command and clears the question on success
   */
  const submitAnswer = useCallback(
    async (response: AskUserQuestionResponse) => {
      if (!activeQuestion) {
        return;
      }

      setIsLoading(true);
      try {
        await api.askUserQuestion.answerQuestion(response);

        clearActiveQuestion();
      } catch {
        // Don't clear question on error so user can retry
      } finally {
        setIsLoading(false);
      }
    },
    [activeQuestion, clearActiveQuestion]
  );

  /**
   * Clear the active question without submitting an answer
   * Use when user dismisses the modal
   */
  const clearQuestion = useCallback(() => {
    clearActiveQuestion();
  }, [clearActiveQuestion]);

  return {
    activeQuestion,
    submitAnswer,
    clearQuestion,
    isLoading,
  };
}
