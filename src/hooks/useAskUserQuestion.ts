/**
 * useAskUserQuestion hook - Handle agent questions requiring user input
 *
 * Listens for agent:ask_user_question Tauri events, stores the question
 * payload in uiStore, and provides functions to submit answers back to
 * the agent.
 */

import { useEffect, useState, useCallback } from "react";
import { useEventBus } from "@/providers/EventProvider";
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
  const eventBus = useEventBus();

  // Set up event listener for agent questions
  useEffect(() => {
    const unsubscribe = eventBus.subscribe<unknown>("agent:ask_user_question", (payload) => {
      // Runtime validation of event payload
      const parsed = AskUserQuestionPayloadSchema.safeParse(payload);

      if (!parsed.success) {
        return;
      }

      setActiveQuestion(parsed.data);
    });

    return unsubscribe;
  }, [setActiveQuestion, eventBus]);

  /**
   * Submit an answer to the agent's question
   * Routes to resolveQuestion (MCP flow) when requestId is present,
   * or answerQuestion (legacy task flow) otherwise.
   */
  const submitAnswer = useCallback(
    async (response: AskUserQuestionResponse) => {
      if (!activeQuestion) {
        return;
      }

      setIsLoading(true);
      try {
        if (response.requestId) {
          await api.askUserQuestion.resolveQuestion({
            requestId: response.requestId,
            selectedOptions: response.selectedOptions,
            ...(response.customResponse !== undefined && { customResponse: response.customResponse }),
          });
        } else {
          await api.askUserQuestion.answerQuestion(response);
        }

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
