/**
 * Ideation store using Zustand with immer middleware
 *
 * Manages ideation session state for the frontend. Sessions are stored in a
 * Record keyed by session ID for O(1) lookup.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { IdeationSession, IdeationSessionStatus } from "@/types/ideation";

// ============================================================================
// State Interface
// ============================================================================

interface IdeationState {
  /** Sessions indexed by ID for O(1) lookup */
  sessions: Record<string, IdeationSession>;
  /** Currently active session ID, or null if none */
  activeSessionId: string | null;
  /** Loading state for async operations */
  isLoading: boolean;
  /** Error message, or null if no error */
  error: string | null;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface IdeationActions {
  /** Set the active session by ID, or null to deselect */
  setActiveSession: (sessionId: string | null) => void;
  /** Add a single session to the store */
  addSession: (session: IdeationSession) => void;
  /** Replace all sessions with new array (converts to Record) */
  setSessions: (sessions: IdeationSession[]) => void;
  /** Update a specific session with partial changes */
  updateSession: (sessionId: string, changes: Partial<IdeationSession>) => void;
  /** Remove a session from the store */
  removeSession: (sessionId: string) => void;
  /** Set loading state */
  setLoading: (isLoading: boolean) => void;
  /** Set error message */
  setError: (error: string | null) => void;
  /** Clear error message */
  clearError: () => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useIdeationStore = create<IdeationState & IdeationActions>()(
  immer((set) => ({
    // Initial state
    sessions: {},
    activeSessionId: null,
    isLoading: false,
    error: null,

    // Actions
    setActiveSession: (sessionId) =>
      set((state) => {
        state.activeSessionId = sessionId;
      }),

    addSession: (session) =>
      set((state) => {
        state.sessions[session.id] = session;
      }),

    setSessions: (sessions) =>
      set((state) => {
        state.sessions = Object.fromEntries(sessions.map((s) => [s.id, s]));
      }),

    updateSession: (sessionId, changes) =>
      set((state) => {
        const session = state.sessions[sessionId];
        if (session) {
          Object.assign(session, changes);
        }
      }),

    removeSession: (sessionId) =>
      set((state) => {
        delete state.sessions[sessionId];
        // Clear active session if removing active session
        if (state.activeSessionId === sessionId) {
          state.activeSessionId = null;
        }
      }),

    setLoading: (isLoading) =>
      set((state) => {
        state.isLoading = isLoading;
      }),

    setError: (error) =>
      set((state) => {
        state.error = error;
      }),

    clearError: () =>
      set((state) => {
        state.error = null;
      }),
  }))
);

// ============================================================================
// Selectors (defined outside store for memoization)
// ============================================================================

/**
 * Select the currently active session
 * @returns The active session, or null if none active
 */
export const selectActiveSession = (
  state: IdeationState & IdeationActions
): IdeationSession | null =>
  state.activeSessionId ? state.sessions[state.activeSessionId] ?? null : null;

/**
 * Select all sessions for a specific project
 * @param projectId - The project ID to filter by
 * @returns Selector function returning matching sessions
 */
export const selectSessionsByProject =
  (projectId: string) =>
  (state: IdeationState): IdeationSession[] =>
    Object.values(state.sessions).filter((s) => s.projectId === projectId);

/**
 * Select all sessions with a specific status
 * @param status - The status to filter by
 * @returns Selector function returning matching sessions
 */
export const selectSessionsByStatus =
  (status: IdeationSessionStatus) =>
  (state: IdeationState): IdeationSession[] =>
    Object.values(state.sessions).filter((s) => s.status === status);
