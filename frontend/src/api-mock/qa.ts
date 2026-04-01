/**
 * Mock QA API
 *
 * Mirrors the interface of src/api/qa-api.ts with mock implementations.
 */

import type { QASettings } from "@/types/qa-config";
import type {
  TaskQAResponse,
  QAResultsResponse,
  UpdateQASettingsInput,
} from "@/api/qa-api";
import { DEFAULT_QA_SETTINGS } from "@/types/qa-config";

// ============================================================================
// Mock QA API
// ============================================================================

export const mockQaApi = {
  getSettings: async (): Promise<QASettings> => {
    return { ...DEFAULT_QA_SETTINGS };
  },

  updateSettings: async (input: UpdateQASettingsInput): Promise<QASettings> => {
    return {
      qa_enabled: input.qa_enabled ?? DEFAULT_QA_SETTINGS.qa_enabled,
      auto_qa_for_ui_tasks: input.auto_qa_for_ui_tasks ?? DEFAULT_QA_SETTINGS.auto_qa_for_ui_tasks,
      auto_qa_for_api_tasks: input.auto_qa_for_api_tasks ?? DEFAULT_QA_SETTINGS.auto_qa_for_api_tasks,
      qa_prep_enabled: input.qa_prep_enabled ?? DEFAULT_QA_SETTINGS.qa_prep_enabled,
      browser_testing_enabled: input.browser_testing_enabled ?? DEFAULT_QA_SETTINGS.browser_testing_enabled,
      browser_testing_url: input.browser_testing_url ?? DEFAULT_QA_SETTINGS.browser_testing_url,
    };
  },

  getTaskQA: async (_taskId: string): Promise<TaskQAResponse | null> => {
    return null;
  },

  getResults: async (_taskId: string): Promise<QAResultsResponse | null> => {
    return null;
  },

  retry: async (taskId: string): Promise<TaskQAResponse> => {
    return {
      id: taskId,
      task_id: taskId,
      screenshots: [],
      created_at: new Date().toISOString(),
    };
  },

  skip: async (taskId: string): Promise<TaskQAResponse> => {
    return {
      id: taskId,
      task_id: taskId,
      screenshots: [],
      created_at: new Date().toISOString(),
    };
  },
} as const;
