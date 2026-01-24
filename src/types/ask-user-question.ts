// AskUserQuestion types and Zod schemas
// For handling agent questions that require user input during execution

import { z } from "zod";

// ========================================
// AskUserQuestion Option
// ========================================

/**
 * An option for the user to select when answering a question
 */
export const AskUserQuestionOptionSchema = z.object({
  label: z.string().min(1),
  description: z.string(),
});

export type AskUserQuestionOption = z.infer<typeof AskUserQuestionOptionSchema>;

// ========================================
// AskUserQuestion Payload
// ========================================

/**
 * Payload sent from agent when it needs user input
 * Rendered as an interactive question component in the UI
 */
export const AskUserQuestionPayloadSchema = z.object({
  taskId: z.string().min(1),
  question: z.string().min(1),
  header: z.string().min(1),
  options: z.array(AskUserQuestionOptionSchema).min(2),
  multiSelect: z.boolean(),
});

export type AskUserQuestionPayload = z.infer<typeof AskUserQuestionPayloadSchema>;

// ========================================
// AskUserQuestion Response
// ========================================

/**
 * Response sent back to agent with user's answer
 */
export const AskUserQuestionResponseSchema = z.object({
  taskId: z.string().min(1),
  selectedOptions: z.array(z.string()),
  customResponse: z.string().optional(),
});

export type AskUserQuestionResponse = z.infer<typeof AskUserQuestionResponseSchema>;

// ========================================
// List Schemas
// ========================================

/**
 * Schema for a list of question payloads
 */
export const AskUserQuestionPayloadListSchema = z.array(AskUserQuestionPayloadSchema);
export type AskUserQuestionPayloadList = z.infer<typeof AskUserQuestionPayloadListSchema>;

// ========================================
// Helper Functions
// ========================================

/**
 * Check if the user selected at least one option
 */
export function hasSelection(response: AskUserQuestionResponse): boolean {
  return response.selectedOptions.length > 0;
}

/**
 * Check if the user provided a custom response
 */
export function hasCustomResponse(response: AskUserQuestionResponse): boolean {
  return response.customResponse !== undefined && response.customResponse.length > 0;
}

/**
 * Check if the response is valid (has selection or custom response)
 */
export function isValidResponse(response: AskUserQuestionResponse): boolean {
  return hasSelection(response) || hasCustomResponse(response);
}

/**
 * Create a response with a single selected option
 */
export function createSingleSelectResponse(
  taskId: string,
  selectedOption: string
): AskUserQuestionResponse {
  return {
    taskId,
    selectedOptions: [selectedOption],
  };
}

/**
 * Create a response with multiple selected options
 */
export function createMultiSelectResponse(
  taskId: string,
  selectedOptions: string[]
): AskUserQuestionResponse {
  return {
    taskId,
    selectedOptions,
  };
}

/**
 * Create a response with a custom text response
 */
export function createCustomResponse(
  taskId: string,
  customResponse: string
): AskUserQuestionResponse {
  return {
    taskId,
    selectedOptions: [],
    customResponse,
  };
}
