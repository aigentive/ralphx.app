// QA API module
// Handles quality assurance settings and task QA operations

import { typedInvoke } from "@/lib/tauri";
import { QASettingsSchema } from "@/types";
import {
  TaskQAResponseSchema,
  QAResultsResponseSchema,
} from "./qa-api.schemas";

// Re-export schemas and types for consumers
export {
  AcceptanceCriterionResponseSchema,
  QATestStepResponseSchema,
  QAStepResultResponseSchema,
  QAResultsResponseSchema,
  TaskQAResponseSchema,
  type AcceptanceCriterionResponse,
  type QATestStepResponse,
  type QAStepResultResponse,
  type QAResultsResponse,
  type TaskQAResponse,
} from "./qa-api.schemas";

/**
 * Input type for updating QA settings (partial update)
 */
export interface UpdateQASettingsInput {
  qa_enabled?: boolean;
  auto_qa_for_ui_tasks?: boolean;
  auto_qa_for_api_tasks?: boolean;
  qa_prep_enabled?: boolean;
  browser_testing_enabled?: boolean;
  browser_testing_url?: string;
}

/**
 * QA API object containing all QA-related Tauri command wrappers
 */
export const qaApi = {
  /**
   * Get global QA settings
   * @returns The current QA settings
   */
  getSettings: () => typedInvoke("get_qa_settings", {}, QASettingsSchema),

  /**
   * Update global QA settings
   * @param input Partial settings to update
   * @returns The updated QA settings
   */
  updateSettings: (input: UpdateQASettingsInput) =>
    typedInvoke("update_qa_settings", { input }, QASettingsSchema),

  /**
   * Get TaskQA data for a specific task
   * @param taskId The task ID
   * @returns TaskQA record or null if none exists
   */
  getTaskQA: (taskId: string) =>
    typedInvoke(
      "get_task_qa",
      { taskId },
      TaskQAResponseSchema.nullable()
    ),

  /**
   * Get QA test results for a specific task
   * @param taskId The task ID
   * @returns QA results or null if no results yet
   */
  getResults: (taskId: string) =>
    typedInvoke(
      "get_qa_results",
      { taskId },
      QAResultsResponseSchema.nullable()
    ),

  /**
   * Retry QA tests for a task
   * Resets test results to pending for re-testing
   * @param taskId The task ID
   * @returns Updated TaskQA record
   */
  retry: (taskId: string) =>
    typedInvoke("retry_qa", { taskId }, TaskQAResponseSchema),

  /**
   * Skip QA for a task
   * Marks all test steps as skipped to bypass QA failure
   * @param taskId The task ID
   * @returns Updated TaskQA record
   */
  skip: (taskId: string) =>
    typedInvoke("skip_qa", { taskId }, TaskQAResponseSchema),
} as const;
