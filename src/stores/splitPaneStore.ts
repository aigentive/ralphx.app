/**
 * Split-pane store using Zustand with immer middleware
 *
 * Manages split-pane layout state for the team view:
 * focused pane, coordinator width, prefix key state, and active context.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

// ============================================================================
// State & Actions
// ============================================================================

interface SplitPaneState {
  /** Currently focused pane — teammate name or "coordinator" */
  focusedPane: string | null;
  /** Coordinator column width as percentage (default 40) */
  coordinatorWidth: number;
  /** Whether the tmux-style prefix key is active */
  isPrefixKeyActive: boolean;
  /** Active team context key (e.g., "task_execution:abc") */
  contextKey: string | null;
}

interface SplitPaneActions {
  setFocusedPane: (pane: string | null) => void;
  setCoordinatorWidth: (width: number) => void;
  setPrefixKeyActive: (active: boolean) => void;
  setContextKey: (key: string | null) => void;
  reset: () => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

const INITIAL_STATE: SplitPaneState = {
  focusedPane: null,
  coordinatorWidth: 40,
  isPrefixKeyActive: false,
  contextKey: null,
};

export const useSplitPaneStore = create<SplitPaneState & SplitPaneActions>()(
  immer((set) => ({
    ...INITIAL_STATE,

    setFocusedPane: (pane) =>
      set((state) => {
        state.focusedPane = pane;
      }),

    setCoordinatorWidth: (width) =>
      set((state) => {
        state.coordinatorWidth = Math.max(20, Math.min(80, width));
      }),

    setPrefixKeyActive: (active) =>
      set((state) => {
        state.isPrefixKeyActive = active;
      }),

    setContextKey: (key) =>
      set((state) => {
        state.contextKey = key;
      }),

    reset: () =>
      set((state) => {
        Object.assign(state, INITIAL_STATE);
      }),
  }))
);

// ============================================================================
// Selectors
// ============================================================================

export const selectFocusedPane = (state: SplitPaneState) => state.focusedPane;
export const selectCoordinatorWidth = (state: SplitPaneState) => state.coordinatorWidth;
export const selectIsPrefixKeyActive = (state: SplitPaneState) => state.isPrefixKeyActive;
export const selectContextKey = (state: SplitPaneState) => state.contextKey;
