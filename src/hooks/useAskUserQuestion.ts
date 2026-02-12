/**
 * useAskUserQuestion hook - Handle agent questions requiring user input
 *
 * Listens for agent:ask_user_question Tauri events, stores per-session
 * question payloads in uiStore, and provides functions to submit answers
 * or dismiss questions.
 */

import { useEffect, useState, useCallback, useRef } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { api } from "@/lib/tauri";
import { useUiStore } from "@/stores/uiStore";
import {
  AskUserQuestionPayloadSchema,
  type AskUserQuestionResponse,
} from "@/types/ask-user-question";

/**
 * Hook to handle agent questions requiring user input, scoped to a session.
 *
 * @param currentSessionId - The session/conversation ID to scope questions to.
 *   When undefined, no question is returned (but events are still stored).
 */
export function useAskUserQuestion(currentSessionId: string | undefined) {
  const [isLoading, setIsLoading] = useState(false);
  const autoDismissTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const activeQuestion = useUiStore((s) =>
    currentSessionId ? (s.activeQuestions[currentSessionId] ?? null) : null
  );
  const answeredQuestion = useUiStore((s) =>
    currentSessionId ? (s.answeredQuestions[currentSessionId] ?? undefined) : undefined
  );

  const setActiveQuestion = useUiStore((s) => s.setActiveQuestion);
  const clearActiveQuestion = useUiStore((s) => s.clearActiveQuestion);
  const dismissQuestionAction = useUiStore((s) => s.dismissQuestion);
  const setAnsweredQuestion = useUiStore((s) => s.setAnsweredQuestion);
  const clearAnsweredQuestion = useUiStore((s) => s.clearAnsweredQuestion);
  const eventBus = useEventBus();

  /**
   * Cancel any pending auto-dismiss timer
   */
  const cancelAutoDismissTimer = useCallback(() => {
    if (autoDismissTimerRef.current) {
      clearTimeout(autoDismissTimerRef.current);
      autoDismissTimerRef.current = null;
    }
  }, []);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      cancelAutoDismissTimer();
    };
  }, [cancelAutoDismissTimer]);

  // Set up event listener for agent questions — stores ALL incoming questions by sessionId
  useEffect(() => {
    const unsubscribe = eventBus.subscribe<unknown>("agent:ask_user_question", (payload) => {
      const parsed = AskUserQuestionPayloadSchema.safeParse(payload);

      if (!parsed.success) {
        console.warn("[useAskUserQuestion] Zod parse failed:", parsed.error.issues);
        return;
      }

      const sessionId = parsed.data.sessionId;
      if (!sessionId) {
        console.warn("[useAskUserQuestion] No sessionId in payload, ignoring");
        return;
      }

      // Cancel any pending auto-dismiss timer for this session (new question arrived)
      if (sessionId === currentSessionId) {
        cancelAutoDismissTimer();
      }

      setActiveQuestion(sessionId, parsed.data);
    });

    return unsubscribe;
  }, [setActiveQuestion, eventBus, currentSessionId, cancelAutoDismissTimer]);

  /**
   * Submit an answer to the agent's question.
   * Routes to resolveQuestion (MCP flow) when requestId is present,
   * or answerQuestion (legacy task flow) otherwise.
   */
  const submitAnswer = useCallback(
    async (response: AskUserQuestionResponse) => {
      if (!activeQuestion || !currentSessionId) {
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

        // Move to answered state
        const summary = response.selectedOptions.length > 0
          ? response.selectedOptions.join(", ")
          : response.customResponse ?? "";
        setAnsweredQuestion(currentSessionId, summary);
        clearActiveQuestion(currentSessionId);

        // Auto-dismiss the answered banner after 3500ms
        cancelAutoDismissTimer();
        autoDismissTimerRef.current = setTimeout(() => {
          clearAnsweredQuestion(currentSessionId);
          autoDismissTimerRef.current = null;
        }, 3500);
      } catch {
        // Don't clear question on error so user can retry
      } finally {
        setIsLoading(false);
      }
    },
    [activeQuestion, currentSessionId, clearActiveQuestion, setAnsweredQuestion, clearAnsweredQuestion, cancelAutoDismissTimer]
  );

  /**
   * Dismiss the question — clears both question and answered state for this session,
   * and sends a dismiss response to the backend so the waiting agent unblocks.
   */
  const dismissQuestion = useCallback(async () => {
    if (!currentSessionId) return;

    const question = activeQuestion;
    dismissQuestionAction(currentSessionId);

    // Cancel any pending auto-dismiss timer
    cancelAutoDismissTimer();

    // If there's an active question with a requestId, send dismiss to backend
    if (question?.requestId) {
      try {
        await api.askUserQuestion.resolveQuestion({
          requestId: question.requestId,
          selectedOptions: [],
          customResponse: "[dismissed]",
        });
      } catch {
        // Best-effort dismiss — don't block UI
      }
    }
  }, [currentSessionId, activeQuestion, dismissQuestionAction, cancelAutoDismissTimer]);

  /**
   * Clear just the answered summary for this session
   */
  const clearAnswered = useCallback(() => {
    if (!currentSessionId) return;
    clearAnsweredQuestion(currentSessionId);
  }, [currentSessionId, clearAnsweredQuestion]);

  return {
    activeQuestion,
    answeredQuestion,
    submitAnswer,
    dismissQuestion,
    clearAnswered,
    isLoading,
  };
}
