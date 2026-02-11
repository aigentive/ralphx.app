// Plan selection and ranking API module
// Wraps Tauri commands for global active plan feature

import { invoke } from "@tauri-apps/api/core";

// ============================================================================
// TypeScript Types
// ============================================================================

export type SelectionSource = "kanban_inline" | "graph_inline" | "quick_switcher" | "ideation";
export interface PlanCandidateResponse {
  sessionId: string;
  title: string | null;
  acceptedAt: string;
  taskStats: {
    total: number;
    incomplete: number;
    activeNow: number;
  };
  interactionStats: {
    selectedCount: number;
    lastSelectedAt: string | null;
  };
  score: number;
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Plan API wrappers for Tauri commands
 * Handles active plan selection and clearing
 */
export const planApi = {
  /**
   * Get the active plan for a project
   * @param projectId The project ID
   * @returns The session ID of the active plan, or null if no plan is active
   */
  getActivePlan: (projectId: string): Promise<string | null> =>
    invoke<string | null>("get_active_plan", { projectId }),

  /**
   * Set the active plan for a project
   * @param projectId The project ID
   * @param sessionId The ideation session ID to set as active
   * @param source The source of the selection (for analytics)
   */
  setActivePlan: (projectId: string, sessionId: string, source: SelectionSource): Promise<void> =>
    invoke("set_active_plan", {
      projectId,
      ideationSessionId: sessionId,
      source,
    }),

  /**
   * Clear the active plan for a project
   * @param projectId The project ID
   */
  clearActivePlan: (projectId: string): Promise<void> =>
    invoke("clear_active_plan", { projectId }),

  /**
   * List accepted plan candidates for selectors
   * @param projectId The project ID
   * @param query Optional search query (title filter)
   */
  listCandidates: (
    projectId: string,
    query?: string
  ): Promise<PlanCandidateResponse[]> =>
    invoke<PlanCandidateResponse[]>("list_plan_selector_candidates", {
      projectId,
      query,
    }),
} as const;
