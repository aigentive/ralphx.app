/**
 * Team store using Zustand with immer middleware
 *
 * Manages team state for agent teams: teammates, messages, costs, streaming text.
 * Keyed by context key (e.g., "task_execution:abc", "session:def") for alignment
 * with chatStore context keys via buildStoreKey().
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { TeamHistoryResponse } from "@/api/team";

// ============================================================================
// Types
// ============================================================================

export type TeammateStatus = "spawning" | "running" | "idle" | "completed" | "failed" | "shutdown";

export interface TeammateState {
  name: string;
  color: string;
  model: string;
  roleDescription: string;
  status: TeammateStatus;
  currentActivity: string | null;
  tokensUsed: number;
  estimatedCostUsd: number;
  streamingText: string;
}

export interface TeamMessage {
  id: string;
  from: string;
  to: string;
  content: string;
  timestamp: string;
}

export interface PendingTeamPlan {
  planId: string;
  process: string;
  teammates: Array<{
    role: string;
    model: string;
    tools: string[];
    mcp_tools: string[];
    prompt_summary: string;
    preset?: string | null;
  }>;
  originContextType: string;
  originContextId: string;
}

interface ActiveTeam {
  teamName: string;
  leadName: string;
  teammates: Record<string, TeammateState>;
  messages: TeamMessage[];
  totalTokens: number;
  totalEstimatedCostUsd: number;
  createdAt: string;
  isHistorical?: boolean | undefined;
}

// ============================================================================
// State & Actions
// ============================================================================

interface TeamState {
  activeTeams: Record<string, ActiveTeam>;
  pendingPlans: Record<string, PendingTeamPlan>;
}

interface TeamActions {
  createTeam: (contextKey: string, teamName: string, leadName: string) => void;
  addTeammate: (contextKey: string, teammate: TeammateState) => void;
  updateTeammateStatus: (contextKey: string, name: string, status: TeammateStatus, activity?: string) => void;
  appendTeammateChunk: (contextKey: string, name: string, text: string) => void;
  clearTeammateStream: (contextKey: string, name: string) => void;
  updateTeammateCost: (contextKey: string, name: string, tokens: number, costUsd: number) => void;
  addTeamMessage: (contextKey: string, message: TeamMessage) => void;
  removeTeammate: (contextKey: string, name: string) => void;
  disbandTeam: (contextKey: string) => void;
  clearTeamForContext: (contextKey: string) => void;
  getTeammates: (contextKey: string) => TeammateState[];
  setPendingPlan: (contextKey: string, plan: PendingTeamPlan) => void;
  clearPendingPlan: (contextKey: string) => void;
  hydrateFromHistory: (contextKey: string, history: TeamHistoryResponse) => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

const MAX_TEAM_MESSAGES = 200;

export const useTeamStore = create<TeamState & TeamActions>()(
  immer((set, get) => ({
    activeTeams: {},
    pendingPlans: {},

    createTeam: (contextKey, teamName, leadName) =>
      set((state) => {
        state.activeTeams[contextKey] = {
          teamName,
          leadName,
          teammates: {},
          messages: [],
          totalTokens: 0,
          totalEstimatedCostUsd: 0,
          createdAt: new Date().toISOString(),
        };
      }),

    addTeammate: (contextKey, teammate) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          team.teammates[teammate.name] = teammate;
        }
      }),

    updateTeammateStatus: (contextKey, name, status, activity) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          const mate = team.teammates[name];
          if (mate) {
            mate.status = status;
            if (activity !== undefined) {
              mate.currentActivity = activity;
            }
          }
        }
      }),

    appendTeammateChunk: (contextKey, name, text) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          const mate = team.teammates[name];
          if (mate) {
            // eslint-disable-next-line no-control-regex
            mate.streamingText += text.replace(/\u001b\[[0-9;]*[A-Za-z]/g, "");
          }
        }
      }),

    clearTeammateStream: (contextKey, name) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          const mate = team.teammates[name];
          if (mate) {
            mate.streamingText = "";
          }
        }
      }),

    updateTeammateCost: (contextKey, name, tokens, costUsd) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          const mate = team.teammates[name];
          if (mate) {
            const tokenDiff = tokens - mate.tokensUsed;
            const costDiff = costUsd - mate.estimatedCostUsd;
            mate.tokensUsed = tokens;
            mate.estimatedCostUsd = costUsd;
            team.totalTokens += tokenDiff;
            team.totalEstimatedCostUsd += costDiff;
          }
        }
      }),

    addTeamMessage: (contextKey, message) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          team.messages.push(message);
          // Cap messages to prevent store bloat
          if (team.messages.length > MAX_TEAM_MESSAGES) {
            team.messages = team.messages.slice(-MAX_TEAM_MESSAGES);
          }
        }
      }),

    removeTeammate: (contextKey, name) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          delete team.teammates[name];
        }
      }),

    disbandTeam: (contextKey) =>
      set((state) => {
        const team = state.activeTeams[contextKey];
        if (team) {
          team.isHistorical = true;
        }
      }),

    clearTeamForContext: (contextKey) =>
      set((state) => {
        delete state.activeTeams[contextKey];
      }),

    getTeammates: (contextKey) => {
      const team = get().activeTeams[contextKey];
      return team ? Object.values(team.teammates) : EMPTY_TEAMMATES;
    },

    setPendingPlan: (contextKey, plan) =>
      set((state) => {
        state.pendingPlans[contextKey] = plan;
      }),

    clearPendingPlan: (contextKey) =>
      set((state) => {
        delete state.pendingPlans[contextKey];
      }),

    hydrateFromHistory: (contextKey, history) =>
      set((state) => {
        // Only hydrate if no active team exists for this context
        if (state.activeTeams[contextKey]) return;
        const session = history.session;
        if (!session) return;

        const teammates: Record<string, TeammateState> = {};
        let totalTokens = 0;
        let totalCostUsd = 0;

        for (const snap of session.teammates) {
          const tokens = snap.cost.input_tokens + snap.cost.output_tokens;
          totalTokens += tokens;
          totalCostUsd += snap.cost.estimated_usd;
          teammates[snap.name] = {
            name: snap.name,
            color: snap.color,
            model: snap.model,
            roleDescription: snap.role,
            status: (snap.status as TeammateStatus) || "shutdown",
            currentActivity: null,
            tokensUsed: tokens,
            estimatedCostUsd: snap.cost.estimated_usd,
            streamingText: "",
          };
        }

        const messages: TeamMessage[] = history.messages.map((m) => ({
          id: m.id,
          from: m.sender,
          to: m.recipient ?? "*",
          content: m.content,
          timestamp: m.createdAt,
        }));

        state.activeTeams[contextKey] = {
          teamName: session.teamName,
          leadName: session.leadName ?? session.teamName,
          teammates,
          messages,
          totalTokens,
          totalEstimatedCostUsd: totalCostUsd,
          createdAt: session.createdAt,
          isHistorical: true,
        };
      }),
  }))
);

// ============================================================================
// Selectors
// ============================================================================

const EMPTY_TEAMMATES: TeammateState[] = [];
/** Exported so consumers use the same ref — avoids selector instability */
export const EMPTY_TEAM_MESSAGES: TeamMessage[] = [];

/**
 * selectTeammates — returns a stable array ref unless teammates actually changed.
 * Uses a closure cache so Object.values() only creates a new array when the
 * underlying Record reference changes (immer produces a new ref on mutation).
 */
export const selectTeammates = (contextKey: string) => {
  let cachedRecord: Record<string, TeammateState> | undefined;
  let cachedResult: TeammateState[] = EMPTY_TEAMMATES;

  return (state: TeamState): TeammateState[] => {
    const team = state.activeTeams[contextKey];
    if (!team) return EMPTY_TEAMMATES;
    // Immer replaces the record ref on mutation — use that as cache key
    if (team.teammates !== cachedRecord) {
      cachedRecord = team.teammates;
      cachedResult = Object.values(team.teammates);
    }
    return cachedResult;
  };
};

export const selectTeamMessages = (contextKey: string) =>
  (state: TeamState): TeamMessage[] =>
    state.activeTeams[contextKey]?.messages ?? EMPTY_TEAM_MESSAGES;

export const selectTeammateByName = (contextKey: string, name: string) =>
  (state: TeamState): TeammateState | null =>
    state.activeTeams[contextKey]?.teammates[name] ?? null;

export const selectActiveTeam = (contextKey: string) =>
  (state: TeamState): ActiveTeam | null =>
    state.activeTeams[contextKey] ?? null;

export const selectIsTeamActive = (contextKey: string) =>
  (state: TeamState): boolean =>
    contextKey in state.activeTeams;

/** Returns true if any team is active across all contexts */
export const selectHasAnyActiveTeam = (state: TeamState): boolean =>
  Object.keys(state.activeTeams).length > 0;

/** Returns total teammate count across all active teams */
export const selectTotalTeammateCount = (state: TeamState): number =>
  Object.values(state.activeTeams).reduce(
    (sum, team) => sum + Object.keys(team.teammates).length,
    0,
  );
