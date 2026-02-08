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
export interface ResolveQuestionInput {
  requestId: string;
  selectedOptions: string[];
  customResponse?: string;
}

export const askUserQuestionApi = {
  /**
   * Submit an answer to an agent's question (legacy task-based flow)
   * @param response The user's response including selected options
   */
  answerQuestion: async (response: AskUserQuestionResponse): Promise<void> => {
    await invoke("answer_user_question", {
      input: {
        taskId: response.taskId,
        selectedOptions: response.selectedOptions,
        customResponse: response.customResponse,
      },
    });
  },

  /**
   * Resolve an MCP-based question by requestId
   * Used when the agent asks questions via the ask_user_question MCP tool
   * @param input The resolution including requestId and selected options
   */
  resolveQuestion: async (input: ResolveQuestionInput): Promise<void> => {
    await invoke("resolve_user_question", {
      args: {
        requestId: input.requestId,
        selectedOptions: input.selectedOptions,
        customResponse: input.customResponse,
      },
    });
  },
} as const;
