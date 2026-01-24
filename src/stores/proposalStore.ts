/**
 * Proposal store using Zustand with immer middleware
 *
 * Manages task proposal state for the frontend. Proposals are stored in a
 * Record keyed by proposal ID for O(1) lookup. The selected state is tracked
 * on each proposal (selected field) with a derived Set available via selector.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { TaskProposal, Priority } from "@/types/ideation";

// ============================================================================
// State Interface
// ============================================================================

interface ProposalState {
  /** Proposals indexed by ID for O(1) lookup */
  proposals: Record<string, TaskProposal>;
  /** Loading state for async operations */
  isLoading: boolean;
  /** Error message, or null if no error */
  error: string | null;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface ProposalActions {
  /** Replace all proposals with new array (converts to Record) */
  setProposals: (proposals: TaskProposal[]) => void;
  /** Add a single proposal to the store */
  addProposal: (proposal: TaskProposal) => void;
  /** Update a specific proposal with partial changes */
  updateProposal: (proposalId: string, changes: Partial<TaskProposal>) => void;
  /** Remove a proposal from the store */
  removeProposal: (proposalId: string) => void;
  /** Toggle selection state of a proposal */
  toggleSelection: (proposalId: string) => void;
  /** Select all proposals in a session */
  selectAll: (sessionId: string) => void;
  /** Deselect all proposals */
  deselectAll: () => void;
  /** Update sort order based on position in array */
  reorder: (proposalIds: string[]) => void;
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

export const useProposalStore = create<ProposalState & ProposalActions>()(
  immer((set) => ({
    // Initial state
    proposals: {},
    isLoading: false,
    error: null,

    // Actions
    setProposals: (proposals) =>
      set((state) => {
        state.proposals = Object.fromEntries(proposals.map((p) => [p.id, p]));
      }),

    addProposal: (proposal) =>
      set((state) => {
        state.proposals[proposal.id] = proposal;
      }),

    updateProposal: (proposalId, changes) =>
      set((state) => {
        const proposal = state.proposals[proposalId];
        if (proposal) {
          Object.assign(proposal, changes);
        }
      }),

    removeProposal: (proposalId) =>
      set((state) => {
        delete state.proposals[proposalId];
      }),

    toggleSelection: (proposalId) =>
      set((state) => {
        const proposal = state.proposals[proposalId];
        if (proposal) {
          proposal.selected = !proposal.selected;
        }
      }),

    selectAll: (sessionId) =>
      set((state) => {
        for (const proposal of Object.values(state.proposals)) {
          if (proposal.sessionId === sessionId) {
            proposal.selected = true;
          }
        }
      }),

    deselectAll: () =>
      set((state) => {
        for (const proposal of Object.values(state.proposals)) {
          proposal.selected = false;
        }
      }),

    reorder: (proposalIds) =>
      set((state) => {
        proposalIds.forEach((id, index) => {
          const proposal = state.proposals[id];
          if (proposal) {
            proposal.sortOrder = index;
          }
        });
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
 * Select all proposals for a specific session
 * @param sessionId - The session ID to filter by
 * @returns Selector function returning matching proposals
 */
export const selectProposalsBySession =
  (sessionId: string) =>
  (state: ProposalState): TaskProposal[] =>
    Object.values(state.proposals).filter((p) => p.sessionId === sessionId);

/**
 * Select all selected proposals
 * @returns All proposals with selected=true
 */
export const selectSelectedProposals = (state: ProposalState): TaskProposal[] =>
  Object.values(state.proposals).filter((p) => p.selected);

/**
 * Get set of selected proposal IDs
 * @returns Set of selected proposal IDs
 */
export const selectSelectedProposalIds = (state: ProposalState): Set<string> =>
  new Set(Object.values(state.proposals).filter((p) => p.selected).map((p) => p.id));

/**
 * Select all proposals with a specific priority
 * @param priority - The priority to filter by
 * @returns Selector function returning matching proposals
 */
export const selectProposalsByPriority =
  (priority: Priority) =>
  (state: ProposalState): TaskProposal[] =>
    Object.values(state.proposals).filter((p) => p.suggestedPriority === priority);

/**
 * Select proposals for a session sorted by sortOrder
 * @param sessionId - The session ID to filter by
 * @returns Selector function returning sorted proposals
 */
export const selectSortedProposals =
  (sessionId: string) =>
  (state: ProposalState): TaskProposal[] =>
    Object.values(state.proposals)
      .filter((p) => p.sessionId === sessionId)
      .sort((a, b) => a.sortOrder - b.sortOrder);
