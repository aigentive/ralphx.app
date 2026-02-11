/**
 * planStore - Stub store for Global Active Plan feature
 *
 * This is a minimal stub implementation to support the PlanQuickSwitcherPalette component.
 * Full implementation will be added when the complete plan feature is implemented.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

// Types (matching the implementation plan)
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

export type SelectionSource = "kanban_inline" | "graph_inline" | "quick_switcher" | "ideation";

interface PlanState {
  activePlanByProject: Record<string, string | null>;
  planCandidates: PlanCandidate[];
  isLoading: boolean;
  error: string | null;
}

interface PlanActions {
  loadActivePlan(projectId: string): Promise<void>;
  setActivePlan(projectId: string, sessionId: string, source: SelectionSource): Promise<void>;
  clearActivePlan(projectId: string): Promise<void>;
  loadCandidates(projectId: string, query?: string): Promise<void>;
}

/**
 * Stub implementation - returns empty data
 * TODO: Replace with full implementation when backend APIs are ready
 */
export const usePlanStore = create<PlanState & PlanActions>()(
  immer((set) => ({
    activePlanByProject: {},
    planCandidates: [],
    isLoading: false,
    error: null,

    loadActivePlan: async () => {
      // Stub: Do nothing
      console.warn("planStore.loadActivePlan: Stub implementation - feature not yet available");
    },

    setActivePlan: async () => {
      // Stub: Do nothing
      console.warn("planStore.setActivePlan: Stub implementation - feature not yet available");
    },

    clearActivePlan: async () => {
      // Stub: Do nothing
      console.warn("planStore.clearActivePlan: Stub implementation - feature not yet available");
    },

    loadCandidates: async () => {
      // Stub: Set empty candidates
      set((state) => {
        state.planCandidates = [];
        state.isLoading = false;
      });
      console.warn("planStore.loadCandidates: Stub implementation - feature not yet available");
    },
  }))
);

// Selectors
export const selectActivePlanId = (projectId: string) => (state: PlanState) =>
  state.activePlanByProject[projectId] ?? null;
