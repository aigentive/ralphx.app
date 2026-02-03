import { describe, it, expect, beforeEach } from "vitest";
import {
  useProposalStore,
  selectProposalsBySession,
  selectProposalsByPriority,
  selectSortedProposals,
} from "./proposalStore";
import type { TaskProposal } from "@/types/ideation";

// Helper to create test proposals
const createTestProposal = (overrides: Partial<TaskProposal> = {}): TaskProposal => ({
  id: `proposal-${Math.random().toString(36).slice(2)}`,
  sessionId: "session-1",
  title: "Test Proposal",
  description: "A test proposal description",
  category: "feature",
  steps: ["Step 1", "Step 2"],
  acceptanceCriteria: ["AC 1"],
  suggestedPriority: "medium",
  priorityScore: 50,
  priorityReason: "Default priority",
  estimatedComplexity: "moderate",
  userPriority: null,
  userModified: false,
  status: "pending",
  createdTaskId: null,
  sortOrder: 0,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  ...overrides,
});

describe("proposalStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useProposalStore.setState({
      proposals: {},
      isLoading: false,
      error: null,
      lastProposalAddedAt: null,
      lastDependencyRefreshRequestedAt: null,
      lastProposalUpdatedAt: null,
      lastUpdatedProposalId: null,
    });
  });

  describe("initial state", () => {
    it("has empty proposals", () => {
      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(0);
    });

    it("has isLoading false", () => {
      const state = useProposalStore.getState();
      expect(state.isLoading).toBe(false);
    });

    it("has null error", () => {
      const state = useProposalStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe("setProposals", () => {
    it("converts array to Record keyed by id", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", title: "Proposal 1" }),
        createTestProposal({ id: "proposal-2", title: "Proposal 2" }),
        createTestProposal({ id: "proposal-3", title: "Proposal 3" }),
      ];

      useProposalStore.getState().setProposals(proposals);

      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(3);
      expect(state.proposals["proposal-1"]?.title).toBe("Proposal 1");
      expect(state.proposals["proposal-2"]?.title).toBe("Proposal 2");
      expect(state.proposals["proposal-3"]?.title).toBe("Proposal 3");
    });

    it("replaces existing proposals", () => {
      useProposalStore.setState({
        proposals: {
          "old-proposal": createTestProposal({ id: "old-proposal", title: "Old" }),
        },
      });

      const newProposals = [createTestProposal({ id: "new-proposal", title: "New" })];
      useProposalStore.getState().setProposals(newProposals);

      const state = useProposalStore.getState();
      expect(state.proposals["old-proposal"]).toBeUndefined();
      expect(state.proposals["new-proposal"]?.title).toBe("New");
    });

    it("handles empty array", () => {
      useProposalStore.getState().setProposals([]);

      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(0);
    });
  });

  describe("addProposal", () => {
    it("adds a proposal to the store", () => {
      const proposal = createTestProposal({ id: "proposal-1" });

      useProposalStore.getState().addProposal(proposal);

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-1"]).toBeDefined();
      expect(state.proposals["proposal-1"]?.title).toBe("Test Proposal");
    });

    it("overwrites proposal with same id", () => {
      const proposal1 = createTestProposal({ id: "proposal-1", title: "First" });
      const proposal2 = createTestProposal({ id: "proposal-1", title: "Second" });

      useProposalStore.getState().addProposal(proposal1);
      useProposalStore.getState().addProposal(proposal2);

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-1"]?.title).toBe("Second");
    });

    it("preserves existing proposals", () => {
      const proposal1 = createTestProposal({ id: "proposal-1" });
      const proposal2 = createTestProposal({ id: "proposal-2" });

      useProposalStore.getState().addProposal(proposal1);
      useProposalStore.getState().addProposal(proposal2);

      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(2);
    });

    it("updates lastProposalAddedAt timestamp when adding proposal", () => {
      const proposal = createTestProposal({ id: "proposal-1" });

      // Initially null
      expect(useProposalStore.getState().lastProposalAddedAt).toBeNull();

      const beforeAdd = Date.now();
      useProposalStore.getState().addProposal(proposal);
      const afterAdd = Date.now();

      const state = useProposalStore.getState();
      expect(state.lastProposalAddedAt).not.toBeNull();
      expect(state.lastProposalAddedAt).toBeGreaterThanOrEqual(beforeAdd);
      expect(state.lastProposalAddedAt).toBeLessThanOrEqual(afterAdd);
    });
  });

  describe("updateProposal", () => {
    it("modifies existing proposal", () => {
      const proposal = createTestProposal({ id: "proposal-1", title: "Original" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().updateProposal("proposal-1", { title: "Updated" });

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-1"]?.title).toBe("Updated");
    });

    it("updates multiple fields", () => {
      const proposal = createTestProposal({
        id: "proposal-1",
        title: "Original",
        suggestedPriority: "medium",
        priorityScore: 50,
      });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().updateProposal("proposal-1", {
        title: "Updated",
        suggestedPriority: "high",
        priorityScore: 75,
      });

      const state = useProposalStore.getState();
      const updated = state.proposals["proposal-1"];
      expect(updated?.title).toBe("Updated");
      expect(updated?.suggestedPriority).toBe("high");
      expect(updated?.priorityScore).toBe(75);
    });

    it("does nothing if proposal not found", () => {
      const proposal = createTestProposal({ id: "proposal-1" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().updateProposal("nonexistent", { title: "Updated" });

      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(1);
      expect(state.proposals["proposal-1"]?.title).toBe("Test Proposal");
    });

    it("preserves other proposal fields", () => {
      const proposal = createTestProposal({
        id: "proposal-1",
        title: "Original",
        description: "A description",
        category: "feature",
      });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().updateProposal("proposal-1", { title: "Updated" });

      const state = useProposalStore.getState();
      const updated = state.proposals["proposal-1"];
      expect(updated?.title).toBe("Updated");
      expect(updated?.description).toBe("A description");
      expect(updated?.category).toBe("feature");
    });
  });

  describe("removeProposal", () => {
    it("removes a proposal from the store", () => {
      const proposal = createTestProposal({ id: "proposal-1" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().removeProposal("proposal-1");

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-1"]).toBeUndefined();
    });

    it("does nothing if proposal not found", () => {
      const proposal = createTestProposal({ id: "proposal-1" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      useProposalStore.getState().removeProposal("nonexistent");

      const state = useProposalStore.getState();
      expect(Object.keys(state.proposals)).toHaveLength(1);
    });
  });

  describe("reorder", () => {
    it("updates sortOrder based on position in array", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", sortOrder: 0 }),
        createTestProposal({ id: "proposal-2", sortOrder: 1 }),
        createTestProposal({ id: "proposal-3", sortOrder: 2 }),
      ];
      useProposalStore.getState().setProposals(proposals);

      // Reorder to: 3, 1, 2
      useProposalStore.getState().reorder(["proposal-3", "proposal-1", "proposal-2"]);

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-3"]?.sortOrder).toBe(0);
      expect(state.proposals["proposal-1"]?.sortOrder).toBe(1);
      expect(state.proposals["proposal-2"]?.sortOrder).toBe(2);
    });

    it("ignores unknown proposal ids", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", sortOrder: 0 }),
        createTestProposal({ id: "proposal-2", sortOrder: 1 }),
      ];
      useProposalStore.getState().setProposals(proposals);

      useProposalStore.getState().reorder(["proposal-2", "nonexistent", "proposal-1"]);

      const state = useProposalStore.getState();
      expect(state.proposals["proposal-2"]?.sortOrder).toBe(0);
      expect(state.proposals["proposal-1"]?.sortOrder).toBe(2);
    });
  });

  describe("setLoading", () => {
    it("sets isLoading to true", () => {
      useProposalStore.getState().setLoading(true);

      const state = useProposalStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets isLoading to false", () => {
      useProposalStore.setState({ isLoading: true });

      useProposalStore.getState().setLoading(false);

      const state = useProposalStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useProposalStore.getState().setError("Something went wrong");

      const state = useProposalStore.getState();
      expect(state.error).toBe("Something went wrong");
    });

    it("clears error with null", () => {
      useProposalStore.setState({ error: "Previous error" });

      useProposalStore.getState().setError(null);

      const state = useProposalStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe("clearError", () => {
    it("clears existing error", () => {
      useProposalStore.setState({ error: "Some error" });

      useProposalStore.getState().clearError();

      const state = useProposalStore.getState();
      expect(state.error).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useProposalStore.setState({
      proposals: {},
      isLoading: false,
      error: null,
      lastProposalAddedAt: null,
      lastDependencyRefreshRequestedAt: null,
      lastProposalUpdatedAt: null,
      lastUpdatedProposalId: null,
    });
  });

  describe("selectProposalsBySession", () => {
    it("returns proposals for matching session", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", sessionId: "session-1" }),
        createTestProposal({ id: "proposal-2", sessionId: "session-2" }),
        createTestProposal({ id: "proposal-3", sessionId: "session-1" }),
      ];
      useProposalStore.getState().setProposals(proposals);

      const selector = selectProposalsBySession("session-1");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((p) => p.id).sort()).toEqual(["proposal-1", "proposal-3"]);
    });

    it("returns empty array when no proposals match", () => {
      const proposal = createTestProposal({ id: "proposal-1", sessionId: "session-1" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      const selector = selectProposalsBySession("session-2");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(0);
    });

    it("returns empty array when store is empty", () => {
      const selector = selectProposalsBySession("session-1");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectProposalsByPriority", () => {
    it("returns proposals with matching priority", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", suggestedPriority: "high" }),
        createTestProposal({ id: "proposal-2", suggestedPriority: "medium" }),
        createTestProposal({ id: "proposal-3", suggestedPriority: "high" }),
      ];
      useProposalStore.getState().setProposals(proposals);

      const selector = selectProposalsByPriority("high");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((p) => p.id).sort()).toEqual(["proposal-1", "proposal-3"]);
    });

    it("returns empty array when no proposals match", () => {
      const proposal = createTestProposal({ id: "proposal-1", suggestedPriority: "low" });
      useProposalStore.setState({ proposals: { "proposal-1": proposal } });

      const selector = selectProposalsByPriority("critical");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectSortedProposals", () => {
    it("returns proposals sorted by sortOrder", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", sessionId: "session-1", sortOrder: 2 }),
        createTestProposal({ id: "proposal-2", sessionId: "session-1", sortOrder: 0 }),
        createTestProposal({ id: "proposal-3", sessionId: "session-1", sortOrder: 1 }),
      ];
      useProposalStore.getState().setProposals(proposals);

      const selector = selectSortedProposals("session-1");
      const result = selector(useProposalStore.getState());

      expect(result.map((p) => p.id)).toEqual(["proposal-2", "proposal-3", "proposal-1"]);
    });

    it("only returns proposals for specified session", () => {
      const proposals = [
        createTestProposal({ id: "proposal-1", sessionId: "session-1", sortOrder: 0 }),
        createTestProposal({ id: "proposal-2", sessionId: "session-2", sortOrder: 0 }),
        createTestProposal({ id: "proposal-3", sessionId: "session-1", sortOrder: 1 }),
      ];
      useProposalStore.getState().setProposals(proposals);

      const selector = selectSortedProposals("session-1");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((p) => p.id)).toEqual(["proposal-1", "proposal-3"]);
    });

    it("returns empty array when store is empty", () => {
      const selector = selectSortedProposals("session-1");
      const result = selector(useProposalStore.getState());

      expect(result).toHaveLength(0);
    });
  });
});
