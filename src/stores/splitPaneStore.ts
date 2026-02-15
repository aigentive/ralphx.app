/**
 * Split-pane store using Zustand with immer middleware
 *
 * Manages split-pane layout state for the team view:
 * focused pane, coordinator width, prefix key state, active context,
 * pane ordering, and per-pane layout/streaming state.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

// ============================================================================
// Types
// ============================================================================

/** Per-pane UI state (layout + streaming indicators) */
export interface PaneState {
  streaming: boolean;
  minimized: boolean;
  maximized: boolean;
  unreadCount: number;
}

// ============================================================================
// State & Actions
// ============================================================================

interface SplitPaneState {
  /** Whether the split-pane team view is active */
  isActive: boolean;
  /** Currently focused pane — teammate name or "coordinator" */
  focusedPane: string | null;
  /** Coordinator column width as percentage (default 40) */
  coordinatorWidth: number;
  /** Whether the tmux-style prefix key is active */
  isPrefixKeyActive: boolean;
  /** Active team context key (e.g., "task_execution:abc") */
  contextKey: string | null;
  /** Ordered pane IDs for display order */
  paneOrder: string[];
  /** Per-pane state keyed by pane ID */
  panes: Record<string, PaneState>;
}

interface SplitPaneActions {
  // Existing
  setFocusedPane: (pane: string | null) => void;
  setCoordinatorWidth: (width: number) => void;
  setPrefixKeyActive: (active: boolean) => void;
  setContextKey: (key: string | null) => void;
  reset: () => void;

  // Lifecycle
  initTeam: (paneIds: string[]) => void;
  addPane: (id: string) => void;
  removePane: (id: string) => void;
  clearTeam: () => void;

  // Navigation
  focusNext: () => void;
  focusPrev: () => void;

  // Layout
  minimizePane: (id: string) => void;
  maximizePane: (id: string) => void;
  restorePane: (id: string) => void;
  resetPaneSizes: () => void;

  // Streaming
  appendPaneChunk: (id: string, chunk: string) => void;
  clearPaneStream: (id: string) => void;
  addPaneToolCall: (id: string, toolCall: unknown) => void;
  addPaneMessage: (id: string, message: unknown) => void;
}

// ============================================================================
// Helpers
// ============================================================================

const DEFAULT_PANE_STATE: PaneState = {
  streaming: false,
  minimized: false,
  maximized: false,
  unreadCount: 0,
};

function createPaneState(): PaneState {
  return { ...DEFAULT_PANE_STATE };
}

// ============================================================================
// Store Implementation
// ============================================================================

const INITIAL_STATE: SplitPaneState = {
  isActive: false,
  focusedPane: null,
  coordinatorWidth: 40,
  isPrefixKeyActive: false,
  contextKey: null,
  paneOrder: [],
  panes: {},
};

export const useSplitPaneStore = create<SplitPaneState & SplitPaneActions>()(
  immer((set) => ({
    ...INITIAL_STATE,

    // ── Existing actions ──────────────────────────────────────────────

    setFocusedPane: (pane) =>
      set((state) => {
        // Clear unread on the newly focused pane
        if (pane !== null && state.panes[pane]) {
          state.panes[pane].unreadCount = 0;
        }
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
        // immer needs explicit reset for nested objects
        state.paneOrder = [];
        state.panes = {};
      }),

    // ── Lifecycle ─────────────────────────────────────────────────────

    initTeam: (paneIds) =>
      set((state) => {
        state.isActive = true;
        state.paneOrder = [...paneIds];
        state.panes = {};
        for (const id of paneIds) {
          state.panes[id] = createPaneState();
        }
        state.focusedPane = paneIds[0] ?? null;
      }),

    addPane: (id) =>
      set((state) => {
        if (!state.paneOrder.includes(id)) {
          state.paneOrder.push(id);
        }
        if (!state.panes[id]) {
          state.panes[id] = createPaneState();
        }
      }),

    removePane: (id) =>
      set((state) => {
        state.paneOrder = state.paneOrder.filter((p) => p !== id);
        delete state.panes[id];
        // Move focus if the removed pane was focused
        if (state.focusedPane === id) {
          state.focusedPane = state.paneOrder[0] ?? null;
        }
      }),

    clearTeam: () =>
      set((state) => {
        state.isActive = false;
        state.paneOrder = [];
        state.panes = {};
        state.focusedPane = null;
      }),

    // ── Navigation ────────────────────────────────────────────────────

    focusNext: () =>
      set((state) => {
        const { paneOrder, focusedPane } = state;
        if (paneOrder.length === 0) return;
        const currentIdx = focusedPane ? paneOrder.indexOf(focusedPane) : -1;
        const nextIdx = currentIdx < paneOrder.length - 1 ? currentIdx + 1 : 0;
        const nextPane = paneOrder[nextIdx];
        if (nextPane !== undefined) {
          if (state.panes[nextPane]) {
            state.panes[nextPane].unreadCount = 0;
          }
          state.focusedPane = nextPane;
        }
      }),

    focusPrev: () =>
      set((state) => {
        const { paneOrder, focusedPane } = state;
        if (paneOrder.length === 0) return;
        const currentIdx = focusedPane ? paneOrder.indexOf(focusedPane) : -1;
        const prevIdx = currentIdx > 0 ? currentIdx - 1 : paneOrder.length - 1;
        const prevPane = paneOrder[prevIdx];
        if (prevPane !== undefined) {
          if (state.panes[prevPane]) {
            state.panes[prevPane].unreadCount = 0;
          }
          state.focusedPane = prevPane;
        }
      }),

    // ── Layout ────────────────────────────────────────────────────────

    minimizePane: (id) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          pane.minimized = true;
          pane.maximized = false;
        }
      }),

    maximizePane: (id) =>
      set((state) => {
        // Maximize one pane → minimize all others
        for (const [paneId, pane] of Object.entries(state.panes)) {
          if (paneId === id) {
            pane.maximized = true;
            pane.minimized = false;
          } else {
            pane.minimized = true;
            pane.maximized = false;
          }
        }
      }),

    restorePane: (id) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          pane.minimized = false;
          pane.maximized = false;
        }
      }),

    resetPaneSizes: () =>
      set((state) => {
        for (const pane of Object.values(state.panes)) {
          pane.minimized = false;
          pane.maximized = false;
        }
        state.coordinatorWidth = 40;
      }),

    // ── Streaming ─────────────────────────────────────────────────────

    appendPaneChunk: (id, _chunk) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          pane.streaming = true;
          // Increment unread if this pane is not focused
          if (state.focusedPane !== id) {
            pane.unreadCount += 1;
          }
        }
      }),

    clearPaneStream: (id) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          pane.streaming = false;
        }
      }),

    addPaneToolCall: (id, _toolCall) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          // Increment unread if this pane is not focused
          if (state.focusedPane !== id) {
            pane.unreadCount += 1;
          }
        }
      }),

    addPaneMessage: (id, _message) =>
      set((state) => {
        const pane = state.panes[id];
        if (pane) {
          // Increment unread if this pane is not focused
          if (state.focusedPane !== id) {
            pane.unreadCount += 1;
          }
        }
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
export const selectIsActive = (state: SplitPaneState) => state.isActive;
export const selectPaneOrder = (state: SplitPaneState) => state.paneOrder;
export const selectPanes = (state: SplitPaneState) => state.panes;
export const selectPaneById = (id: string) => (state: SplitPaneState) => state.panes[id] ?? null;
