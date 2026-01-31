/**
 * Mock Ask User Question API
 *
 * Provides mock implementation for ask-user-question operations.
 * Used for browser testing and visual regression testing.
 */

import type { AskUserQuestionResponse } from "@/types/ask-user-question";

/**
 * Mock Ask User Question API matching the real API interface
 */
export const mockAskUserQuestionApi = {
  /**
   * Mock answer submission - no-op for visual testing
   * In web mode, agent questions are simulated via events
   */
  answerQuestion: async (_response: AskUserQuestionResponse): Promise<void> => {
    // No-op - visual testing doesn't process answers
    console.log("[mock] answerQuestion called");
  },
} as const;
