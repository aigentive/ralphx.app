/**
 * Mock Test Data API
 *
 * Mirrors the interface of src/api/test-data.ts with mock implementations.
 */

import type { SeedResponse, TestDataProfile } from "@/api/test-data";
import { resetStore } from "./store";

// ============================================================================
// Mock Test Data API
// ============================================================================

export const mockTestDataApi = {
  seed: async (profile?: TestDataProfile): Promise<SeedResponse> => {
    resetStore();
    return {
      profile: profile ?? "kanban",
      projectId: "project-mock-1",
      projectName: "Demo Project",
      tasksCreated: 11, // 8 status tasks + 3 extra
      sessionsCreated: 1,
      proposalsCreated: 1,
    };
  },

  seedVisualAudit: async (): Promise<SeedResponse> => {
    resetStore();
    return {
      profile: "kanban",
      projectId: "project-mock-1",
      projectName: "Demo Project",
      tasksCreated: 11,
      sessionsCreated: 1,
      proposalsCreated: 1,
    };
  },

  clear: async (): Promise<string> => {
    resetStore();
    return "Mock data cleared";
  },
} as const;
