/**
 * Ideation store using Zustand with immer middleware
 *
 * Manages ideation session state for the frontend. Sessions are stored in a
 * Record keyed by session ID for O(1) lookup.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { IdeationSession, IdeationSessionStatus } from "@/types/ideation";
import type { Artifact } from "@/types/artifact";
import type { IdeationSettings } from "@/types/ideation-config";

// ============================================================================
// Constants
// ============================================================================

/** Maximum number of sessions to keep in memory before LRU eviction */
const MAX_CACHED_SESSIONS = 20;

// ============================================================================
// State Interface
// ============================================================================

export interface ProactiveSyncNotification {
  /** Artifact ID that was updated */
  artifactId: string;
  /** Proposal IDs that may need updating */
  proposalIds: string[];
  /** Previous proposal states for undo functionality */
  previousStates: Record<string, unknown>;
  /** Timestamp when notification was created */
  timestamp: number;
}

interface IdeationState {
  /** Sessions indexed by ID for O(1) lookup */
  sessions: Record<string, IdeationSession>;
  /** Currently active session ID, or null if none */
  activeSessionId: string | null;
  /** Plan artifact for the active session, or null if none */
  planArtifact: Artifact | null;
  /** Ideation settings */
  ideationSettings: IdeationSettings | null;
  /** Proactive sync notification for stale proposals */
  syncNotification: ProactiveSyncNotification | null;
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
  /** Select a session atomically (adds to store + sets active in one update) */
  selectSession: (session: IdeationSession) => void;
  /** Replace all sessions with new array (converts to Record) */
  setSessions: (sessions: IdeationSession[]) => void;
  /** Update a specific session with partial changes */
  updateSession: (sessionId: string, changes: Partial<IdeationSession>) => void;
  /** Remove a session from the store */
  removeSession: (sessionId: string) => void;
  /** Set the plan artifact for the active session */
  setPlanArtifact: (artifact: Artifact | null) => void;
  /** Fetch the plan artifact for a given artifact ID */
  fetchPlanArtifact: (artifactId: string) => Promise<void>;
  /** Set ideation settings */
  setIdeationSettings: (settings: IdeationSettings) => void;
  /** Show proactive sync notification */
  showSyncNotification: (notification: ProactiveSyncNotification) => void;
  /** Dismiss sync notification */
  dismissSyncNotification: () => void;
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
    planArtifact: null,
    ideationSettings: null,
    syncNotification: null,
    isLoading: false,
    error: null,

    // Actions
    setActiveSession: (sessionId) =>
      set((state) => {
        const isSameSession = state.activeSessionId === sessionId;
        state.activeSessionId = sessionId;
        // Clear session-specific state only when switching to a different session
        if (!isSameSession) {
          state.planArtifact = null;
          state.syncNotification = null;
          state.error = null;
        }
      }),

    addSession: (session) =>
      set((state) => {
        state.sessions[session.id] = session;
        // LRU eviction: remove oldest session if over limit
        const sessionIds = Object.keys(state.sessions);
        if (sessionIds.length > MAX_CACHED_SESSIONS) {
          // Find the oldest session (by updatedAt) that's not the active session
          const oldest = sessionIds
            .filter((id) => id !== state.activeSessionId)
            .sort((a, b) => {
              const aTime = new Date(state.sessions[a]?.updatedAt ?? 0).getTime();
              const bTime = new Date(state.sessions[b]?.updatedAt ?? 0).getTime();
              return aTime - bTime;
            })[0];
          if (oldest) {
            delete state.sessions[oldest];
          }
        }
      }),

    selectSession: (session) =>
      set((state) => {
        const isSameSession = state.activeSessionId === session.id;
        // Add session to store
        state.sessions[session.id] = session;

        // Set as active
        state.activeSessionId = session.id;

        // Clear session-specific state only when switching sessions
        if (!isSameSession) {
          state.planArtifact = null;
          state.syncNotification = null;
          state.error = null;
        }

        // LRU eviction if needed
        const sessionIds = Object.keys(state.sessions);
        if (sessionIds.length > MAX_CACHED_SESSIONS) {
          const oldest = sessionIds
            .filter((id) => id !== session.id)
            .sort((a, b) => {
              const aTime = new Date(state.sessions[a]?.updatedAt ?? 0).getTime();
              const bTime = new Date(state.sessions[b]?.updatedAt ?? 0).getTime();
              return aTime - bTime;
            })[0];
          if (oldest) {
            delete state.sessions[oldest];
          }
        }
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

    setPlanArtifact: (artifact) =>
      set((state) => {
        state.planArtifact = artifact;
      }),

    fetchPlanArtifact: async (artifactId) => {
      const { artifactApi } = await import("@/api/artifact");
      try {
        set((state) => {
          state.isLoading = true;
          state.error = null;
        });
        const artifact = await artifactApi.get(artifactId);
        set((state) => {
          state.planArtifact = artifact;
          state.isLoading = false;
        });
      } catch (error) {
        set((state) => {
          state.error =
            error instanceof Error ? error.message : "Failed to fetch plan artifact";
          state.isLoading = false;
        });
      }
    },

    setIdeationSettings: (settings) =>
      set((state) => {
        state.ideationSettings = settings;
      }),

    showSyncNotification: (notification) =>
      set((state) => {
        state.syncNotification = notification;
      }),

    dismissSyncNotification: () =>
      set((state) => {
        state.syncNotification = null;
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
