/**
 * Ask User Question API Module
 *
 * Provides a centralized API wrapper for answering agent questions.
 * This module follows the domain API pattern used by other centralized modules.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AskUserQuestionResponse } from "@/types/ask-user-question";

// ============================================================================
// Ask User Question API Object
// ============================================================================

/**
 * Ask User Question API object containing typed Tauri command wrappers
 */
export const askUserQuestionApi = {
  /**
   * Submit an answer to an agent's question
   * @param response The user's response including selected options
   */
  answerQuestion: async (response: AskUserQuestionResponse): Promise<void> => {
    await invoke("answer_user_question", {
      input: {
        task_id: response.taskId,
        selected_options: response.selectedOptions,
        custom_response: response.customResponse,
      },
    });
    // Command returns () on success, no parsing needed
  },
} as const;
