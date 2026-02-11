/**
 * Plan store using Zustand with immer middleware
 *
 * Manages active plan state per project and cached plan candidates.
 * The active plan determines which tasks are displayed in Graph and Kanban views.
 * Uses Record<projectId, sessionId | null> for O(1) lookup.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { planApi, type SelectionSource } from "@/api/plan";

// ============================================================================
// State Interface
// ============================================================================

interface PlanState {
  /** Active plan session ID by project ID (projectId → sessionId | null) */
  activePlanByProject: Record<string, string | null>;
  /** Cached plan candidates (from last loadCandidates call) */
  planCandidates: PlanCandidate[];
  /** Loading state for async operations */
  isLoading: boolean;
  /** Error message from last failed operation */
  error: string | null;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface PlanActions {
  /** Load active plan for a project from backend */
  loadActivePlan: (projectId: string) => Promise<void>;
  /** Set active plan for a project (with source tracking) */
  setActivePlan: (
    projectId: string,
    sessionId: string,
    source: SelectionSource
  ) => Promise<void>;
  /** Clear active plan for a project */
  clearActivePlan: (projectId: string) => Promise<void>;
  /** Load plan candidates for selector (not implemented yet - will call list_plan_selector_candidates) */
  loadCandidates: (projectId: string, query?: string) => Promise<void>;
}

// ============================================================================
// Types
// ============================================================================


export interface PlanCandidate {
  sessionId: string;
  title: string | null;
  acceptedAt: string;
  taskStats: TaskStats;
  interactionStats: InteractionStats;
  score: number;
}

export interface TaskStats {
  total: number;
  incomplete: number;
  activeNow: number;
}

export interface InteractionStats {
  selectedCount: number;
  lastSelectedAt: string | null;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const usePlanStore = create<PlanState & PlanActions>()(
  immer((set) => ({
    // Initial state
    activePlanByProject: {},
    planCandidates: [],
    isLoading: false,
    error: null,

    // Actions
    loadActivePlan: async (projectId) => {
      try {
        set({ isLoading: true, error: null });
        const sessionId = await planApi.getActivePlan(projectId);
        set((state) => {
          state.activePlanByProject[projectId] = sessionId;
          state.isLoading = false;
        });
      } catch (error) {
        set({
          error: error instanceof Error ? error.message : "Failed to load active plan",
          isLoading: false,
        });
      }
    },

    setActivePlan: async (projectId, sessionId, source) => {
      try {
        set({ isLoading: true, error: null });
        await planApi.setActivePlan(projectId, sessionId, source);
        set((state) => {
          state.activePlanByProject[projectId] = sessionId;
          state.isLoading = false;
        });
      } catch (error) {
        set({
          error: error instanceof Error ? error.message : "Failed to set active plan",
          isLoading: false,
        });
        throw error; // Re-throw so callers can handle
      }
    },

    clearActivePlan: async (projectId) => {
      try {
        set({ isLoading: true, error: null });
        await planApi.clearActivePlan(projectId);
        set((state) => {
          state.activePlanByProject[projectId] = null;
          state.isLoading = false;
        });
      } catch (error) {
        set({
          error: error instanceof Error ? error.message : "Failed to clear active plan",
          isLoading: false,
        });
        throw error;
      }
    },

    loadCandidates: async (_projectId, _query) => {
      // Placeholder - will be implemented when list_plan_selector_candidates backend command is ready
      set({ isLoading: true, error: null });
      try {
        // TODO: Call planApi.listCandidates(_projectId, _query) when backend command is ready
        console.warn("loadCandidates not yet implemented - waiting for backend");
        set({ planCandidates: [], isLoading: false });
      } catch (error) {
        set({
          error: error instanceof Error ? error.message : "Failed to load candidates",
          isLoading: false,
        });
      }
    },
  }))
);

// ============================================================================
// Selectors (defined outside store for memoization)
// ============================================================================

/**
 * Select the active plan ID for a specific project
 * @param projectId - The project ID to look up
 * @returns Selector function returning the session ID or null
 */
export const selectActivePlanId =
  (projectId: string) =>
  (state: PlanState): string | null =>
    state.activePlanByProject[projectId] ?? null;

/**
 * Select the active plan for the current active project
 * Requires the state to include activeProjectId (from project store)
 * @returns The active plan session ID or null
 */
export const selectCurrentActivePlan = (
  state: PlanState & { activeProjectId: string | null }
): string | null =>
  state.activeProjectId
    ? state.activePlanByProject[state.activeProjectId] ?? null
    : null;
