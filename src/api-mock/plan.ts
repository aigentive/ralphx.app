/**
 * Mock Plan API
 *
 * Mirrors src/api/plan.ts with mock implementations.
 */

import { getStore } from "./store";
import type { SelectionSource } from "@/api/plan";

export const mockPlanApi = {
  /**
   * Get the active plan (session ID) for a project
   */
  getActivePlan: async (projectId: string): Promise<string | null> => {
    const store = getStore();
    return store.activePlans?.get(projectId) ?? null;
  },

  /**
   * Set the active plan for a project
   */
  setActivePlan: async (
    projectId: string,
    sessionId: string,
    _source: SelectionSource
  ): Promise<void> => {
    const store = getStore();
    if (!store.activePlans) {
      store.activePlans = new Map();
    }
    store.activePlans.set(projectId, sessionId);
  },

  /**
   * Clear the active plan for a project
   */
  clearActivePlan: async (projectId: string): Promise<void> => {
    const store = getStore();
    if (!store.activePlans) {
      store.activePlans = new Map();
    }
    store.activePlans.set(projectId, null);
  },

  /**
   * List plan selector candidates
   * Returns empty array for now - not implemented in tests
   */
  listCandidates: async (_projectId: string, _query?: string): Promise<unknown[]> => {
    return [];
  },
} as const;
