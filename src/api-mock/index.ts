/**
 * Mock API Module
 *
 * Provides a mock implementation of the Tauri API for browser testing.
 * Mirrors the interface of src/lib/tauri.ts api object.
 *
 * Usage:
 * - List/get operations return factory-generated mock data
 * - Create/update/delete operations return success (no-op for read-only mode)
 * - Sufficient for visual regression testing, styling checks, layout verification
 */

import { mockTasksApi, mockStepsApi } from "./tasks";
import { mockProjectsApi, mockWorkflowsApi } from "./projects";
import { mockMethodologiesApi } from "./methodologies";
import { mockArtifactsApi } from "./artifact";
import { mockResearchApi } from "./research";
import { mockAskUserQuestionApi } from "./ask-user-question";
import { mockExecutionApi } from "./execution";
import { mockReviewsApi, mockFixTasksApi } from "./reviews";
import { mockQaApi } from "./qa";
import { mockTestDataApi } from "./test-data";

// Re-export for direct imports
export { mockTasksApi, mockStepsApi } from "./tasks";
export { mockProjectsApi, mockWorkflowsApi, mockGetGitBranches } from "./projects";
export { mockMethodologiesApi } from "./methodologies";
export { mockArtifactsApi } from "./artifact";
export { mockResearchApi } from "./research";
export { mockAskUserQuestionApi } from "./ask-user-question";
export { mockExecutionApi } from "./execution";
export { mockChatApi } from "./chat";
export { mockIdeationApi } from "./ideation";
export { mockReviewsApi, mockFixTasksApi } from "./reviews";
export { mockQaApi } from "./qa";
export { mockActivityEventsApi } from "./activity-events";
export { mockArtifactApi } from "./artifact";
export { mockTestDataApi } from "./test-data";
export { getStore, resetStore } from "./store";

/**
 * Aggregate mock API object matching the structure of the real API
 *
 * This mirrors the api object from src/lib/tauri.ts
 */
export const mockApi = {
  health: {
    check: async () => ({ status: "ok" }),
  },

  tasks: mockTasksApi,
  projects: mockProjectsApi,
  workflows: mockWorkflowsApi,
  methodologies: mockMethodologiesApi,
  artifacts: mockArtifactsApi,
  research: mockResearchApi,
  askUserQuestion: mockAskUserQuestionApi,
  qa: mockQaApi,
  reviews: mockReviewsApi,
  fixTasks: mockFixTasksApi,
  execution: mockExecutionApi,
  steps: mockStepsApi,
  testData: mockTestDataApi,
} as const;

/**
 * Type alias for the mock API (matches real API shape)
 */
export type MockApi = typeof mockApi;
